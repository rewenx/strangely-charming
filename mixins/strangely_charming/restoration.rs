use super::{Game, sc_runes::Pos};
use horizon_world::{
    nbt::Tag,
    worldgen::structure::{Mirror, Rotation},
};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
const RUBY: &str = "strangely_charming:restoration_ruby";
const PREFIX: &str = "strangely_charming:runestone_";
const LOST: &str = "strangely_charming:broken_carvings";
const AIR: &str = "minecraft:air";
const BRICKS: &str = "minecraft:stone_bricks";
pub(super) const CHANNEL: &str = "strangely_charming:restore/";
type World = horizon_client::world::World;
fn socket_key([x, y, z]: Pos) -> i32 {
    (y << 8) | ((x & 15) << 4) | (z & 15)
}
// Save provenance in the same chunk snapshot as the block mutation.
pub(super) fn mark_socket(w: &mut World, p: Pos, lost: bool) {
    let Some(c) = w.chunks.get_mut(&(p[0] >> 4, p[2] >> 4)) else {
        return;
    };
    let extras = Arc::make_mut(Arc::make_mut(c).saved.get_or_insert_with(Default::default));
    let entry = extras
        .root
        .entry(LOST.into())
        .or_insert_with(|| Tag::IntArray(Vec::new()));
    if let Tag::IntArray(v) = entry {
        let key = socket_key(p);
        if lost {
            if !v.contains(&key) {
                v.push(key)
            }
        } else {
            v.retain(|k| *k != key)
        }
    }
    w.modified.insert((p[0] >> 4, p[2] >> 4));
}
fn lost(w: &World, p: Pos) -> bool {
    w.chunks
        .get(&(p[0] >> 4, p[2] >> 4))
        .and_then(|c| c.saved.as_ref())
        .and_then(|v| v.root.get(&LOST.into()))
        .is_some_and(|v| matches!(v,Tag::IntArray(a) if a.contains(&socket_key(p))))
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
struct Circle {
    lo: Pos,
    hi: Pos,
    caps: Vec<(Pos, u8)>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Ritual {
    circle: Circle,
    owner: String,
    due: i64,
}
#[derive(Clone)]
struct Offer {
    p: Pos,
    dim: String,
    nonce: u64,
    expires: Instant,
    circle: Circle,
}
thread_local! {
    static OFFERS:RefCell<HashMap<(PathBuf,u128),Offer>>=RefCell::default();
    static POLL:RefCell<HashMap<PathBuf,(i64,Vec<PathBuf>)>>=RefCell::default();
}
fn string(t: &Tag, key: &str) -> Option<String> {
    if let Tag::String(v) = t.get(key)? {
        v.to_string().ok()
    } else {
        None
    }
}
fn int(t: &Tag, key: &str) -> Option<i32> {
    if let Tag::Int(v) = t.get(key)? {
        Some(*v)
    } else {
        None
    }
}
fn circle_id(id: &str) -> bool {
    matches!(
        id,
        "strangely_charming:rune_circle"
            | "strangely_charming:rune_circle_trial"
            | "strangely_charming:rune_circle_stronghold"
            | "strangely_charming:rune_circle_ancient_city"
    )
}
fn piece(start: &Tag, p: Pos) -> Option<(String, Pos, Rotation, Pos, Pos)> {
    let Tag::List(pieces) = start.get("Children")? else {
        return None;
    };
    for c in pieces {
        let Tag::IntArray(bb) = c.get("BB")? else {
            continue;
        };
        let &[x, y, z, xx, yy, zz] = bb.as_slice() else {
            continue;
        };
        if !(x..=xx).contains(&p[0])
            || !(y..=yy).contains(&p[1])
            || !(z..=zz).contains(&p[2])
            || xx - x > 40
            || zz - z > 40
            || yy - y > 32
        {
            continue;
        }
        let id = string(c.get("pool_element")?, "location")?;
        if !id.starts_with("strangely_charming:circle/") {
            continue;
        }
        let rot = match string(c, "rotation")?.as_str() {
            "NONE" => Rotation::None,
            "CLOCKWISE_90" => Rotation::Clockwise90,
            "CLOCKWISE_180" => Rotation::Clockwise180,
            "COUNTERCLOCKWISE_90" => Rotation::Counterclockwise90,
            _ => continue,
        };
        return Some((
            id,
            [int(c, "PosX")?, int(c, "PosY")?, int(c, "PosZ")?],
            rot,
            [x, y, z],
            [xx, yy, zz],
        ));
    }
    None
}
fn gravity(n: &str) -> bool {
    matches!(
        n,
        "minecraft:gravel"
            | "minecraft:sand"
            | "minecraft:red_sand"
            | "minecraft:suspicious_sand"
            | "minecraft:suspicious_gravel"
    ) || n.ends_with("_concrete_powder")
}
fn protected(n: &str) -> bool {
    horizon_client::lan::chunks::block_entity_type(n).is_some()
        || super::sc_graves::is_marker(n)
        || n == "strangely_charming:kiln"
        || matches!(
            n,
            "minecraft:bedrock"
                | "minecraft:barrier"
                | "minecraft:end_portal"
                | "minecraft:end_portal_frame"
                | "minecraft:nether_portal"
                | "minecraft:reinforced_deepslate"
                | "minecraft:lodestone"
                | "minecraft:respawn_anchor"
                | "minecraft:spawner"
                | "minecraft:trial_spawner"
                | "minecraft:vault"
        )
        || n.ends_with("_bed")
        || n.starts_with("strangely_charming:rune_portal_")
}
impl Circle {
    fn contains(&self, p: Pos) -> bool {
        (0..3).all(|i| (self.lo[i]..=self.hi[i]).contains(&p[i]))
    }
    fn valid(&self, g: &Game<'_>, p: Pos) -> bool {
        self.caps.iter().any(|(q, id)| {
            *q == p
                && !lost(&g.world, p)
                && g.block_name(p)
                    .strip_prefix(PREFIX)
                    .and_then(|s| s.parse::<u8>().ok())
                    == Some(*id)
        })
    }
    fn intact(&self, g: &Game<'_>) -> bool {
        self.caps.iter().all(|(p, _)| self.valid(g, *p))
    }
    fn loaded(&self, g: &Game<'_>) -> bool {
        ((self.lo[0] >> 4)..=(self.hi[0] >> 4)).all(|cx| {
            ((self.lo[2] >> 4)..=(self.hi[2] >> 4)).all(|cz| g.world.chunks.contains_key(&(cx, cz)))
        })
    }
    fn plan(&self, g: &Game<'_>) -> Result<Vec<(Pos, String)>, &'static str> {
        if !self.loaded(g) {
            return Err("Come closer so the whole circle is loaded.");
        }
        let [west, north, east, south] = g.border.bounds();
        if f64::from(self.lo[0]) < west
            || f64::from(self.lo[2]) < north
            || f64::from(self.hi[0]) + 1. > east
            || f64::from(self.hi[2]) + 1. > south
        {
            return Err("The circle crosses the world border.");
        }
        let floor = self
            .caps
            .iter()
            .map(|(p, _)| p[1])
            .min()
            .ok_or("No original carving remains.")?
            - 2;
        let center_x = (self.lo[0] + self.hi[0]).div_euclid(2);
        let center_z = (self.lo[2] + self.hi[2]).div_euclid(2);
        let bottom = self.lo[1].min(floor);
        if bottom < g.world.bottom_y() || self.hi[1] >= g.world.top_y() {
            return Err("The circle crosses the world's build limit.");
        }
        let mut plan = Vec::new();
        for x in self.lo[0]..=self.hi[0] {
            for z in self.lo[2]..=self.hi[2] {
                let cap = self.caps.iter().find(|(q, _)| q[0] == x && q[2] == z);
                let radius_squared = (x - center_x).pow(2) + (z - center_z).pow(2);
                let inlay = (7..=12).contains(&radius_squared);
                for y in (bottom..g.world.top_y()).rev() {
                    let p = [x, y, z];
                    let old = g.block_name(p);
                    let new = if y > self.hi[1] {
                        if !gravity(&old) && !g.items().block_in_tag("minecraft:leaves", &old) {
                            continue;
                        }
                        AIR.into()
                    } else if y == floor && inlay {
                        "strangely_charming:amethyst_4".into()
                    } else if y <= floor {
                        BRICKS.into()
                    } else if let Some((q, id)) = cap
                        && y <= q[1]
                    {
                        if y == q[1] {
                            format!("{PREFIX}{id}")
                        } else {
                            BRICKS.into()
                        }
                    } else {
                        AIR.into()
                    };
                    if old.as_ref() == new
                        || new == AIR && g.res.facts().is_air(g.world.block(x, y, z))
                    {
                        continue;
                    }
                    if protected(&old)
                        || g.world.block_entities.contains_key(&(x, y, z))
                        || super::spill::saved_entity(&g.world, p).is_some()
                    {
                        return Err(
                            "Move the protected block or container before restoring this circle.",
                        );
                    }
                    plan.push((p, new));
                }
            }
        }
        Ok(plan)
    }
}
impl Game<'_> {
    fn sc_find_circle(&mut self, p: Pos) -> Option<Circle> {
        if self.dim.id() != "minecraft:overworld" {
            return None;
        }
        let mut seed = None;
        'chunks: for cx in (p[0] >> 4) - 2..=(p[0] >> 4) + 2 {
            for cz in (p[2] >> 4) - 2..=(p[2] >> 4) + 2 {
                let Some(c) = self.world.chunks.get(&(cx, cz)) else {
                    continue;
                };
                for s in &c.structure_starts {
                    if circle_id(&s.name)
                        && let Some(t) = s.save()
                        && let Some(v) = piece(&t, p)
                    {
                        seed = Some(v);
                        break 'chunks;
                    }
                }
                if let Some(v) = c
                    .saved
                    .as_ref()
                    .and_then(|v| v.root.get(&"structures".into()))
                    .and_then(|v| v.get("starts"))
                    && let Tag::Compound(starts) = v
                {
                    for (k, t) in starts {
                        if circle_id(&k.to_string_lossy())
                            && let Some(v) = piece(t, p)
                        {
                            seed = Some(v);
                            break 'chunks;
                        }
                    }
                }
            }
        }
        let (id, at, rot, lo, hi) = seed?;
        // Resolve the saved piece against this mod's shipped template, not a similarly
        // named structure in the world's generated/ directory.
        use horizon_world::{
            compression::Compression,
            nbt::{self, Limits},
            worldgen::structure::StructureTemplate,
        };
        let path = format!(
            "data/strangely_charming/structure/{}.nbt",
            id.strip_prefix("strangely_charming:")?
        );
        let (_, bytes) = horizon_mods::strangely_charming_content::FILES
            .iter()
            .find(|(k, _)| *k == path)?;
        let raw = Compression::Gzip.decode(bytes, 2 << 20).ok()?;
        let (tag, _) = nbt::read_named(&raw, Limits::templates()).ok()?;
        let t = StructureTemplate::from_nbt(
            &tag.tag,
            self.pack.assets.registry,
            self.state_transforms()?,
        );
        let mut caps = Vec::new();
        for b in t.palettes().first()?.blocks() {
            if let Some(id) = self
                .res
                .facts()
                .get(b.state)
                .name
                .strip_prefix(PREFIX)
                .and_then(|n| n.parse::<u8>().ok())
                .filter(|n| *n < 26)
            {
                let rel = StructureTemplate::transform(b.pos, Mirror::None, rot, [0; 3]);
                caps.push((std::array::from_fn(|i| at[i] + rel[i]), id));
            }
        }
        caps.sort_unstable();
        let c = Circle { lo, hi, caps };
        (c.caps.len() == 8 && c.caps.iter().all(|(q, _)| c.contains(*q)) && c.valid(self, p))
            .then_some(c)
    }
    fn sc_ritual_dir(&self) -> PathBuf {
        self.sc_rune_dir()
            .join("repairs")
            .join(self.dim.id().replace(':', "_"))
    }
    fn sc_ritual_file(&self, c: &Circle) -> PathBuf {
        let [x, y, z] = c.lo;
        self.sc_ritual_dir().join(format!("{x}_{y}_{z}.json"))
    }
    fn sc_in_reach(&self, p: Pos) -> bool {
        self.eye_position().is_some_and(|e| {
            (0..3)
                .map(|i| (e[i] - p[i] as f64 - 0.5).powi(2))
                .sum::<f64>()
                <= 49.
        })
    }
    pub(super) fn sc_restore_click(&mut self, p: Pos) -> bool {
        if !horizon_mods::evanders_config::enabled("rune_circle_repair")
            || !self.held().is_some_and(|s| s.item.as_ref() == RUBY)
            || !self.block_name(p).starts_with(PREFIX)
        {
            return false;
        }
        if self.remote.is_some() || !self.may_build() || self.spectator() {
            return true;
        }
        let Some(c) = self.sc_find_circle(p) else {
            self.sc_notice("Only a surviving original rune can restore its circle.");
            return true;
        };
        if self.sc_ritual_file(&c).exists() {
            self.sc_notice("This circle is already being restored.");
            return true;
        }
        if c.intact(self) {
            self.sc_notice("this circle is fully intact");
            return true;
        }
        if let Err(e) = c.plan(self) {
            self.sc_notice(e);
            return true;
        }
        use horizon_core::random::RandomSource;
        let nonce = self.world.random.next_long() as u64;
        OFFERS.with_borrow_mut(|v| {
            v.retain(|_, o| o.expires > Instant::now());
            v.insert(
                (self.world_root.clone(), self.native_acting_id()),
                Offer {
                    p,
                    dim: self.dim.id().into(),
                    nonce,
                    expires: Instant::now() + Duration::from_secs(90),
                    circle: c,
                },
            );
        });
        let button = |label: &str, answer: &str| {
            Tag::compound([
                ("label", Tag::String(label.into())),
                (
                    "action",
                    Tag::compound([
                        ("type", Tag::String("minecraft:custom".into())),
                        (
                            "id",
                            Tag::String(format!("{CHANNEL}{nonce:016x}/{answer}").as_str().into()),
                        ),
                    ]),
                ),
            ])
        };
        self.show_click_dialog(&Tag::compound([
            ("type", Tag::String("minecraft:multi_action".into())),
            ("title", Tag::String("Restoration Ruby".into())),
            (
                "body",
                Tag::compound([
                    ("type", Tag::String("minecraft:plain_message".into())),
                    (
                        "contents",
                        Tag::String("This will void blocks inside the circle. Continue?".into()),
                    ),
                ]),
            ),
            (
                "actions",
                Tag::List(vec![button("Yes", "yes"), button("No", "no")]),
            ),
            ("columns", Tag::Int(2)),
            ("pause", Tag::Byte(0)),
        ]));
        true
    }
    pub(super) fn sc_offer_bounds(&mut self) -> Option<(Pos, Pos)> {
        OFFERS.with_borrow(|v| {
            v.get(&(self.world_root.clone(), self.native_acting_id()))
                .map(|o| (o.circle.lo, o.circle.hi))
        })
    }
    pub(super) fn sc_restore_answer(&mut self, a: &str) {
        if !horizon_mods::evanders_config::enabled("rune_circle_repair") {
            return;
        }
        let Some((n, yes)) = a.split_once('/') else {
            return;
        };
        let Ok(n) = u64::from_str_radix(n, 16) else {
            return;
        };
        let owner = self.native_acting_id();
        let key = (self.world_root.clone(), owner);
        let Some(o) = OFFERS.with_borrow_mut(|v| {
            if v.get(&key).is_some_and(|o| o.nonce == n) {
                v.remove(&key)
            } else {
                None
            }
        }) else {
            return;
        };
        if yes != "yes" {
            self.sc_notice("restoral abandoned");
            return;
        }
        if !self.may_build()
            || self.spectator()
            || o.expires < Instant::now()
            || self.dim.id() != o.dim
            || !self.sc_in_reach(o.p)
            || !self.held().is_some_and(|s| s.item.as_ref() == RUBY)
        {
            self.sc_notice("Hold the ruby beside the original rune and try again.");
            return;
        }
        let c = o.circle;
        if !c.valid(self, o.p) {
            self.sc_notice("The original rune is gone.");
            return;
        }
        if c.intact(self) {
            self.sc_notice("this circle is fully intact");
            return;
        }
        if let Err(e) = c.plan(self) {
            self.sc_notice(e);
            return;
        }
        let path = self.sc_ritual_file(&c);
        if path.exists() {
            self.sc_notice("This circle is already being restored.");
            return;
        }
        let job = Ritual {
            circle: c,
            owner: format!("{owner:032x}"),
            due: self.world.game_time + 100,
        };
        if super::sc_runes::save(&path, &job).is_err() {
            self.sc_notice("The restoration could not be saved.");
            return;
        }
        POLL.with_borrow_mut(|v| {
            v.remove(&self.sc_ritual_dir());
        });
        let stack = self.held_mut();
        if let Some(s) = stack {
            s.count -= 1;
            if s.count == 0 {
                *stack = None
            }
        }
        let w = &mut self.world.weather;
        w.clear_time = 0;
        w.rain_time = 6000;
        w.thunder_time = 6000;
        w.raining = true;
        w.thundering = true;
        self.sc_notice("You hear thunder overhead...");
    }
    pub(super) fn sc_repair_tick(&mut self) {
        if !horizon_mods::evanders_config::enabled("rune_circle_repair") || self.remote.is_some() {
            return;
        }
        let dir = self.sc_ritual_dir();
        let time = self.world.game_time;
        let paths = POLL.with_borrow_mut(|v| {
            let e = v.entry(dir.clone()).or_insert_with(|| {
                (
                    i64::MIN,
                    std::fs::read_dir(&dir)
                        .into_iter()
                        .flatten()
                        .flatten()
                        .map(|x| x.path())
                        .filter(|p| p.extension().is_some_and(|s| s == "json"))
                        .collect(),
                )
            });
            if e.1.is_empty() || e.0 == time {
                return Vec::new();
            }
            e.0 = time;
            e.1.clone()
        });
        for path in paths {
            let Ok(Some(job)) = super::sc_runes::read::<Ritual>(&path) else {
                continue;
            };
            if time < job.due || !job.circle.loaded(self) {
                continue;
            }
            let c = &job.circle;
            let owner = u128::from_str_radix(&job.owner, 16).unwrap_or(0);
            let notice = |g: &mut Game<'_>, s: &str| {
                if g.native_acting_id() == owner {
                    g.sc_notice(s)
                } else {
                    let mut data = vec![0];
                    data.extend_from_slice(s.as_bytes());
                    g.native_send(
                        owner,
                        horizon_protocol::packets::play::Clientbound::CustomPayload(
                            horizon_protocol::packets::configuration::CustomPayload {
                                channel: super::sc_alerts::CHANNEL.into(),
                                data,
                            },
                        ),
                    );
                }
            };
            let plan = match c.plan(self) {
                Ok(plan) => plan,
                Err(e) => {
                    if self.native_acting_id() != owner {
                        continue;
                    }
                    if std::fs::remove_file(&path).is_ok() {
                        self.give_or_drop(super::sc_travel::stack(RUBY));
                        notice(self, e);
                        POLL.with_borrow_mut(|v| {
                            v.remove(&dir);
                        });
                    }
                    continue;
                }
            };
            if !c.caps.iter().any(|(p, _)| c.valid(self, *p)) {
                if self.native_acting_id() != owner {
                    continue;
                }
                if std::fs::remove_file(&path).is_ok() {
                    self.give_or_drop(super::sc_travel::stack(RUBY));
                    POLL.with_borrow_mut(|v| {
                        v.remove(&dir);
                    });
                    notice(self, "No original rune remains. Your ruby was returned.");
                }
                continue;
            }
            if !plan.iter().all(|(p, n)| self.sc_grave_set(*p, n)) {
                continue;
            }
            for (p, _) in &c.caps {
                mark_socket(&mut self.world, *p, false)
            }
            if std::fs::remove_file(&path).is_err() {
                continue;
            }
            POLL.with_borrow_mut(|v| {
                v.remove(&dir);
            });
            for i in [0, 3, 6] {
                let p = c.caps[i].0;
                let at = [p[0] as f64 + 0.5, p[1] as f64 + 1., p[2] as f64 + 0.5];
                self.spawn_bolt(at, true);
                if let Some(l) = self.native_level(self.dim.id()) {
                    use horizon_client::mod_api::{ParticleOptions, ParticleType};
                    self.native_level_send_particles(
                        l,
                        &ParticleOptions::simple(ParticleType::CampfireCosySmoke),
                        None,
                        at,
                        [0.45, 0.5, 0.45],
                        0.02,
                        12,
                    );
                }
            }
            notice(self, "the rune circle has been fully restored!");
        }
    }
}
