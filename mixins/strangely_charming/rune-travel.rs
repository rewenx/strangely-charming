use super::{Game, HudBatch, rgba};
use horizon_core::random::RandomSource;
use horizon_protocol::packets::{configuration::CustomPayload, play::Clientbound};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fs, io,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock, mpsc},
    time::{Duration, Instant},
};

pub(super) fn manual_facing(yaw: f32, pitch: f32) -> &'static str {
    if pitch >= 65. {
        return "up";
    }
    if pitch <= -65. {
        return "down";
    }
    let [x, _, z] = super::view_vector(yaw, 0.);
    if x.abs() > z.abs() {
        if x > 0. { "west" } else { "east" }
    } else if z > 0. {
        "north"
    } else {
        "south"
    }
}

pub(super) const PEARL: &str = "strangely_charming:rune_pearl";
thread_local! {
    pub(super) static RUNE_IMPACT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
pub(super) struct RuneImpact;
impl Drop for RuneImpact {
    fn drop(&mut self) {
        RUNE_IMPACT.set(false);
    }
}
pub(super) fn is_rune_pearl(p: &horizon_client::projectiles::Projectile) -> bool {
    matches!(
        p.kind,
        horizon_client::projectiles::ProjectileKind::EnderPearl
    ) && p.item.as_ref().is_some_and(|s| s.item.as_ref() == PEARL)
}
pub(super) const VIEW: &str = "strangely_charming:rune_research";
pub(super) fn enabled() -> bool {
    horizon_mods::evanders_config::enabled("rune_travel")
}
use horizon_client::stations::sc_glyphs::{self, NAMES};
pub(super) type Pos = [i32; 3];
pub(super) type Address = [u8; 12];
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct Site {
    pub dim: String,
    pub pos: Pos,
}
impl Site {
    fn new(dim: &str, pos: Pos) -> Self {
        Self {
            dim: dim.into(),
            pos,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Journey {
    pub from: Site,
    pub to: Site,
    #[serde(default)]
    pub icon: Option<TripIcon>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(super) enum TripIcon {
    Mishap,
    Carving(u8),
    Amethyst(u8),
}
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Research {
    pub learned: Vec<u8>,
    pub names: HashMap<u8, String>,
    pub trips: VecDeque<Journey>,
    pub all_recipes: bool,
}
impl Research {
    pub fn name(&self, id: u8) -> &str {
        self.names
            .get(&id)
            .map(String::as_str)
            .unwrap_or(NAMES[id as usize])
    }
    fn learn(&mut self, id: u8) -> bool {
        if self.learned.contains(&id) {
            return false;
        }
        self.learned.push(id);
        true
    }
    fn log(&mut self, from: Site, to: Site, icon: TripIcon) {
        self.trips.push_front(Journey {
            from,
            to,
            icon: Some(icon),
        });
        self.trips.truncate(5)
    }
}
pub(super) fn glyph(id: &str, prefix: &str) -> Option<u8> {
    id.strip_prefix(prefix)?
        .parse::<u8>()
        .ok()
        .filter(|n| *n < 26)
}
pub(super) fn read<T: DeserializeOwned>(path: &Path) -> io::Result<Option<T>> {
    match fs::read(path) {
        Ok(b) => Ok(Some(serde_json::from_slice(&b)?)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}
pub(super) fn save<T: Serialize>(path: &Path, v: &T) -> io::Result<()> {
    fs::create_dir_all(path.parent().unwrap())?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, serde_json::to_vec(v)?)?;
    fs::rename(tmp, path)
}
fn vanilla_dimension(dim: &str) -> bool {
    matches!(
        dim,
        "minecraft:overworld" | "minecraft:the_nether" | "minecraft:the_end"
    )
}
fn file_key(site: &Site) -> String {
    format!(
        "{}_{}_{}_{}",
        site.dim.replace(':', "_"),
        site.pos[0],
        site.pos[1],
        site.pos[2]
    )
}
fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(14695981039346656037, |v, b| {
        (v ^ u64::from(*b)).wrapping_mul(1099511628211)
    })
}
fn digits(mut key: u64) -> Address {
    let mut out = [0; 12];
    for n in out.iter_mut().rev() {
        *n = (key % 26) as u8;
        key /= 26;
    }
    out
}
fn code(a: Address) -> String {
    a.iter().map(|n| char::from(b'a' + *n)).collect()
}
// A shared diagram may only reproduce an address already bound to its recorded destination.
pub(super) fn registered(root: &Path, glyphs: Address, site: &Site) -> bool {
    glyphs.iter().all(|g| *g < 26)
        && vanilla_dimension(&site.dim)
        && read::<Site>(&root.join("addresses").join(code(glyphs)))
            .ok()
            .flatten()
            .as_ref()
            == Some(site)
}
// Looking at the same twelve blocks from behind exchanges the two uprights
// and the ends of both horizontal rows. The top and bottom centers stay put.
fn reflected(a: Address) -> Address {
    [8, 7, 6, 5, 4, 3, 2, 1, 0, 11, 10, 9].map(|i| a[i])
}
pub(super) fn address(root: &Path, site: &Site) -> io::Result<Address> {
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap();
    // Strange used BlockPos#toLong + the signed dimension ID, then read the
    // magnitude in base 26. Keep that layout; probe on collisions instead of
    // replacing another journal's destination.
    let [x, y, z] = site.pos;
    let packed = ((i64::from(x) & 0x3ff_ffff) << 38)
        | ((i64::from(z) & 0x3ff_ffff) << 12)
        | (i64::from(y) & 0xfff);
    let mut key = match site.dim.as_str() {
        "minecraft:overworld" => packed.unsigned_abs(),
        "minecraft:the_nether" => packed.wrapping_sub(1).unsigned_abs(),
        "minecraft:the_end" => packed.wrapping_add(1).unsigned_abs(),
        _ => hash(file_key(site).as_bytes()),
    };
    let registry = root.join("addresses");
    loop {
        let a = digits(key);
        let file = registry.join(code(a));
        match read::<Site>(&file)? {
            Some(other) if &other != site => key = key.wrapping_add(1),
            Some(_) => return Ok(a),
            None => {
                // A reservation is implicit in its one destination file, so it
                // cannot get out of sync with a separately stored mirror alias.
                let other_side = reflected(a);
                if other_side != a && registry.join(code(other_side)).try_exists()? {
                    key = key.wrapping_add(1);
                    continue;
                }
                save(&file, site)?;
                return Ok(a);
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Outcome {
    Nothing,
    Stray,
    Travel,
    Learn,
    TwoCreepers,
    Zombie,
    Blast,
    Lightning,
    Eureka,
    Known,
    Survey,
}
pub(super) fn outcome(pearl: bool, roll: i32) -> Outcome {
    use Outcome::*;
    if pearl {
        match roll {
            0..=79 => Learn,
            80..=97 => Travel,
            _ => Eureka,
        }
    } else {
        match roll {
            0..=24 => Nothing,
            25..=34 => Stray,
            35..=59 => Travel,
            60..=74 => Learn,
            75..=79 => TwoCreepers,
            80..=84 => Zombie,
            85..=94 => Blast,
            _ => Lightning,
        }
    }
}
fn pearl_outcome(enhanced: bool, roll: i32, known: bool) -> Outcome {
    if !known {
        outcome(enhanced, roll)
    } else if !enhanced && roll < 25 {
        Outcome::Nothing
    } else {
        Outcome::Known
    }
}
thread_local! {pub(super) static CLIENT:RefCell<Research>=RefCell::default();pub(super) static DIAGRAMS:RefCell<HashMap<String,Address>>=RefCell::default();}
pub(super) fn clear() {
    sc_glyphs::clear();
    HINT.with_borrow_mut(|h| *h = Hint::default());
    CLIENT.with_borrow_mut(|v| *v = Research::default());
    DIAGRAMS.with_borrow_mut(HashMap::clear);
}
pub(super) fn receive(data: &[u8]) {
    if data.len() > 16000 {
        return;
    }
    if let Ok((r, a)) = serde_json::from_slice::<(Research, Option<HashMap<String, Address>>)>(data)
    {
        if r.learned.len() > 26 || r.learned.iter().any(|v| *v >= 26) || r.trips.len() > 5 {
            return;
        }
        sc_glyphs::update(&r.learned, &r.names);
        CLIENT.with_borrow_mut(|v| *v = r);
        if let Some(a) = a {
            DIAGRAMS.with_borrow_mut(|v| *v = a);
        }
    }
}
struct Trip {
    rune: u8,
    what: Outcome,
    enhanced: bool,
    from: Site,
    started: Instant,
    last_tick: Instant,
    next_try: Instant,
    anchor: Option<Pos>,
    retries: u32,
    ready: mpsc::Receiver<Option<Pos>>,
    target: Option<Pos>,
    route: Option<PathBuf>,
    source: Site,
    cached: Option<Pos>,
}
type Trips = HashMap<(PathBuf, u128), Trip>;
static TRIPS: OnceLock<Mutex<Trips>> = OnceLock::new();
const TYPES: &[&str] = &[
    "spawn",
    "strangely_charming:rune_circle",
    "minecraft:village_plains",
    "minecraft:pillager_outpost",
    "minecraft:desert_pyramid",
    "minecraft:jungle_pyramid",
    "minecraft:mineshaft",
    "minecraft:ocean_ruin_cold",
    "minecraft:swamp_hut",
    "minecraft:igloo",
    "strangely_charming:underground_ruins",
    "minecraft:trial_chambers",
    "minecraft:ancient_city",
];
fn category(seed: i64, rune: u8) -> &'static str {
    let mut kinds = std::array::from_fn::<_, 26, _>(|n| TYPES[n % TYPES.len()]);
    let mut h = hash(&seed.to_le_bytes());
    for i in (1..26).rev() {
        h = h.wrapping_mul(6364136223846793005).wrapping_add(1);
        kinds.swap(i, (h as usize) % (i + 1));
    }
    kinds[rune as usize]
}
// Avoid re-reading the same research file or resending it every few seconds.
// A new connection receives its HUD data even before opening a journal.
type SessionMap = HashMap<(PathBuf, u128), (Option<u32>, Instant)>;
thread_local! {static SESSIONS:RefCell<SessionMap>=RefCell::default();}
impl Game<'_> {
    pub(super) fn sc_rune_dir(&self) -> PathBuf {
        self.world_root.join("strangely_charming").join("runes")
    }
    pub(super) fn sc_owner_mask(&self, owner: u128) -> u32 {
        let path = self
            .sc_rune_dir()
            .join("players")
            .join(format!("{owner:032x}.json"));
        read::<Research>(&path)
            .ok()
            .flatten()
            .map_or(0, |r| sc_glyphs::mask(&r.learned))
    }
    fn sc_research_path(&mut self) -> PathBuf {
        self.sc_rune_dir()
            .join("players")
            .join(format!("{:032x}.json", self.native_acting_id()))
    }
    pub(super) fn sc_research(&mut self) -> io::Result<Research> {
        Ok(read(&self.sc_research_path())?.unwrap_or_default())
    }
    fn sc_store_research(&mut self, r: &Research) -> bool {
        if let Err(e) = save(&self.sc_research_path(), r) {
            log::error!("Rune research save: {e}");
            self.sc_notice("Your rune research could not be saved.");
            return false;
        }
        self.sc_send_research(r, None);
        true
    }
    pub(super) fn sc_rune_command(&mut self, line: &str, actor: Option<u128>) -> Option<String> {
        let mut words = line.trim_start_matches('/').split_whitespace();
        if words.next()? != "scrunes" {
            return None;
        }
        let result = (|| -> Result<String, String> {
            if !enabled() {
                return Err("Rune travel is disabled.".into());
            }
            if words.next() != Some("unlock-all") {
                return Err("Usage: /scrunes unlock-all [online player]".into());
            }
            let name = words.next().unwrap_or("@s");
            if words.next().is_some() {
                return Err("Usage: /scrunes unlock-all [online player]".into());
            }
            let players = self.native_server_players();
            let player = if name == "@s" {
                let who = actor.ok_or("Supply an online player name from the console.")?;
                players
                    .iter()
                    .find(|p| p.uuid == who)
                    .ok_or("Player is no longer online.")?
            } else {
                players
                    .iter()
                    .find(|p| p.name.eq_ignore_ascii_case(name))
                    .ok_or_else(|| format!("No online player named {name}."))?
            };
            let target = self
                .target_by_uuid(player.uuid)
                .ok_or("Player is no longer online.")?;
            self.for_player(target, |g| {
                if g.native_acting_id() != player.uuid {
                    return Err("Could not access that player's rune research.".into());
                }
                let mut research = g.sc_research().map_err(|e| {
                    log::error!("Rune grant read: {e}");
                    "Could not read that player's rune research."
                })?;
                let added = (0..NAMES.len() as u8)
                    .filter(|&id| research.learn(id))
                    .count();
                if added == 0 {
                    return Ok(format!("{} already knows all 26 runes.", player.name));
                }
                if !g.sc_store_research(&research) {
                    return Err("Could not save that player's rune research.".into());
                }
                Ok(format!(
                    "Unlocked {added} new {} for {}. All 26 runes are now known.",
                    if added == 1 { "rune" } else { "runes" },
                    player.name
                ))
            })
            .ok_or("Could not access that player.")?
        })();
        Some(result.unwrap_or_else(|e| e))
    }
    pub(super) fn sc_flag_recipes(&mut self) {
        if let Ok(mut r) = self.sc_research()
            && !r.all_recipes
        {
            r.all_recipes = true;
            self.sc_store_research(&r);
        }
    }
    pub(super) fn sc_rename_rune(&mut self, id: usize, name: &str) {
        if !enabled()
            || id >= 26
            || name.trim().is_empty()
            || name.chars().any(char::is_control)
            || name.encode_utf16().count() > 24
        {
            return;
        }
        if let Ok(mut r) = self.sc_research()
            && r.learned.contains(&(id as u8))
        {
            r.names.insert(id as u8, name.trim().to_string());
            self.sc_store_research(&r);
        }
    }
    fn sc_send_research(&mut self, r: &Research, a: Option<HashMap<String, Address>>) {
        let key = (self.sc_rune_dir(), self.native_acting_id());
        WATCHES.with_borrow_mut(|w| w.entry(key).or_default().mask = sc_glyphs::mask(&r.learned));
        let data = serde_json::to_vec(&(r, a)).unwrap();
        if self.lan_acting.is_some() {
            let who = self.native_acting_id();
            self.native_send(
                who,
                Clientbound::CustomPayload(CustomPayload {
                    channel: VIEW.into(),
                    data,
                }),
            );
        } else {
            receive(&data);
        }
    }
    pub(super) fn sc_book_runes(&mut self, book: &super::sc_travel::Book) {
        if !enabled() {
            return;
        }
        let dir = self.sc_rune_dir();
        let mut a = HashMap::new();
        for e in &book.entries {
            match address(&dir, &Site::new(&e.dim, e.pos)) {
                Ok(v) => {
                    a.insert(e.id.clone(), v);
                }
                Err(err) => log::error!("Could not register journal address: {err}"),
            }
        }
        if let Ok(r) = self.sc_research() {
            self.sc_send_research(&r, Some(a));
        }
    }
    pub(super) fn sc_throw_rune(&mut self, stack: &horizon_client::items::ItemStack) -> bool {
        let mut inherited = self.known_player_movement();
        if self.player.as_ref().is_some_and(|p| p.ground.on_ground) {
            inherited.y = 0.;
        }
        let before = self.creatures.projectiles.len();
        if self.throw_item(stack).unwrap_or(false)
            && let Some(p) = self.creatures.projectiles.get_mut(before)
        {
            // Keep the thrower's motion and the vanilla spread, increasing only launch speed.
            p.velocity.x = inherited.x + (p.velocity.x - inherited.x) * 1.5;
            p.velocity.y = inherited.y + (p.velocity.y - inherited.y) * 1.5;
            p.velocity.z = inherited.z + (p.velocity.z - inherited.z) * 1.5;
            let mut shown = stack.clone();
            shown.count = 1;
            p.item = Some(shown);
        }
        true
    }
    pub(super) fn sc_rune_impact(&mut self, cap: Pos, enhanced: bool) -> bool {
        if !enabled() || self.remote.is_some() || !vanilla_dimension(self.dim.id()) {
            return false;
        }
        let Some(rune) = glyph(&self.block_name(cap), "strangely_charming:runestone_") else {
            return false;
        };
        let Some(p) = self
            .player
            .as_ref()
            .filter(|p| !p.health.dead_or_dying() && self.sleeping.is_none())
        else {
            return true;
        };
        let at = [p.position.x, p.position.y, p.position.z];
        let from = Site::new(self.dim.id(), at.map(|x| x.floor() as i32));
        let roll = self.world.random.next_int_bound(100).unwrap();
        let Ok(research) = self.sc_research() else {
            self.sc_notice("Your rune research could not be read.");
            return true;
        };
        let what = pearl_outcome(enhanced, roll, research.learned.contains(&rune));
        use Outcome::*;
        match what {
            Nothing => self.sc_notice("it had no effect"),
            TwoCreepers | Zombie => {
                let bad = what == TwoCreepers;
                let hp = if bad { 2.0 } else { 4.0 };
                if let Some(p) = self.player.as_mut() {
                    let _ = p.set_health(p.health.health().min(hp), false);
                }
                for offset in if bad { &[-2, 2][..] } else { &[2][..] } {
                    let candidates = [[*offset, 0], [0, *offset], [-*offset, 0], [0, -*offset]];
                    let mut spawn = at;
                    for [dx, dz] in candidates {
                        if let Some(s) =
                            self.sc_safe_column(from.pos[0] + dx, from.pos[2] + dz, from.pos[1])
                        {
                            spawn = s.map(f64::from);
                            spawn[0] += 0.5;
                            spawn[2] += 0.5;
                            break;
                        }
                    }
                    self.spawn_mob(
                        if horizon_mods::evanders_config::enabled("protogen_mishaps") {
                            "strangely_charming:protogen"
                        } else if bad {
                            "minecraft:creeper"
                        } else {
                            "minecraft:zombie"
                        },
                        spawn,
                        false,
                    );
                }
                self.sc_notice(if bad { "Ahhh!!" } else { "oh no!" });
            }
            Lightning => {
                self.spawn_bolt(at, false);
                self.sc_notice("ouch");
            }
            Blast => {
                self.sc_notice("ouch");
                let interaction = self.destroy_type("tnt_explosion_drop_decay");
                let _ =
                    self.server_explode_as(at, 4., interaction, false, "minecraft:explosion", None);
            }
            _ => {
                if let Err(err) = self.sc_start_trip(rune, what, from, cap, enhanced) {
                    log::warn!("Rune travel: {err}");
                    self.sc_notice("The gateway could not open its travel records.");
                }
            }
        }
        true
    }
    fn sc_safe_column(&self, x: i32, z: i32, near: i32) -> Option<Pos> {
        if !self.world.chunks.contains_key(&(x >> 4, z >> 4)) {
            return None;
        }
        let low = self.world.bottom_y() + 1;
        let high = self.world.top_y() - 2;
        let base = near.clamp(low, high);
        for d in 0..=(high - low) {
            for y in [base + d, base - d] {
                if y < low || y > high {
                    continue;
                }
                let floor = self.block_name([x, y - 1, z]);
                if matches!(
                    floor.as_ref(),
                    "minecraft:lava"
                        | "minecraft:water"
                        | "minecraft:magma_block"
                        | "minecraft:powder_snow"
                        | "minecraft:cactus"
                        | "minecraft:campfire"
                ) {
                    continue;
                }
                let ground = self.res.facts().get(self.world.block(x, y - 1, z));
                if ground.solid_render
                    && [y, y + 1]
                        .iter()
                        .all(|v| self.res.facts().is_air(self.world.block(x, *v, z)))
                {
                    return Some([x, y, z]);
                }
            }
        }
        None
    }
    fn sc_top_landing(&self, x: i32, z: i32) -> Option<Pos> {
        if !self.world.loaded(x, z) {
            return None;
        }
        let facts = self.res.facts();
        for y in (self.world.bottom_y()..self.world.top_y()).rev() {
            let state = self.world.block(x, y, z);
            if facts.is_air(state) {
                continue;
            }
            let floor = facts.get(state);
            if matches!(
                floor.name.as_ref(),
                "minecraft:water"
                    | "minecraft:lava"
                    | "minecraft:bubble_column"
                    | "minecraft:powder_snow"
                    | "minecraft:magma_block"
                    | "minecraft:cactus"
                    | "minecraft:campfire"
                    | "minecraft:soul_campfire"
                    | "minecraft:fire"
                    | "minecraft:soul_fire"
                    | "minecraft:sweet_berry_bush"
                    | "minecraft:wither_rose"
                    | "minecraft:pointed_dripstone"
            ) {
                return None;
            }
            if floor.fluid != horizon_client::blocks::Fluid::None && !floor.full_collision {
                return None;
            }
            // We have scanned everything above this block: only air or dry,
            // collision-free cover may be above the player. This also permits
            // leaves, snow layers and partial supports such as stairs.
            let height = if floor.full_collision {
                1.0
            } else {
                let collision = self.res.collision().cached_shapes();
                let shape = collision.cached_shape(state).ok()??;
                if shape.is_empty() {
                    continue;
                }
                let height = shape
                    .boxes()
                    .iter()
                    .filter(|b| {
                        b.min_position().x < 0.8
                            && b.max_position().x > 0.2
                            && b.min_position().z < 0.8
                            && b.max_position().z > 0.2
                    })
                    .map(|b| b.max_position().y)
                    .fold(0f64, f64::max);
                if height <= 0. {
                    return None;
                }
                height
            };
            let feet = y + height.ceil() as i32;
            // Build height limits block placement. The space above the top layer is open.
            return Some([x, feet, z]);
        }
        None
    }
    fn sc_start_trip(
        &mut self,
        rune: u8,
        what: Outcome,
        from: Site,
        cap: Pos,
        enhanced: bool,
    ) -> io::Result<()> {
        let root = self.sc_rune_dir();
        let key = (root.clone(), self.native_acting_id());
        let mut trips = TRIPS.get_or_init(Default::default).lock().unwrap();
        trips.retain(|_, v| v.last_tick.elapsed() < Duration::from_secs(90));
        if trips.get(&key).is_some_and(|active| {
            active.what == Outcome::Survey && (what != Outcome::Survey || active.source.pos != cap)
        }) {
            trips.remove(&key);
        }
        if trips.contains_key(&key) {
            if what == Outcome::Survey {
                return Ok(());
            }
            self.sc_notice("Your previous gateway is still opening.");
            return Ok(());
        }
        let (tx, rx) = mpsc::channel();
        let source = Site::new(&from.dim, cap);
        let route = (what != Outcome::Stray).then(|| root.join("pillars").join(file_key(&source)));
        let cached = match &route {
            Some(p) => read::<Pos>(p)?,
            None => None,
        };
        let fallback = self
            .level
            .spawn
            .unwrap_or([0., 64., 0.])
            .map(|v| v.floor() as i32);
        if let Some(c) = cached {
            let _ = tx.send(Some(c));
        } else if what == Outcome::Stray {
            let angle = self.world.random.next_double() * std::f64::consts::TAU;
            let radius = self.world.random.next_double().sqrt() * 396.;
            let _ = tx.send(Some([
                from.pos[0] + (radius * angle.cos()) as i32,
                from.pos[1],
                from.pos[2] + (radius * angle.sin()) as i32,
            ]));
        } else {
            let id = category(self.level.seed, rune);
            let terrain = self.end.locator();
            let seed = hash(file_key(&source).as_bytes());
            let start = [
                cap[0] + ((seed >> 16) % 3601) as i32 - 1800,
                cap[1],
                cap[2] + (seed % 3601) as i32 - 1800,
            ];
            std::thread::spawn(move || {
                let p = if id == "spawn" {
                    fallback
                } else {
                    terrain
                        .and_then(|t| t.locate(&[id.to_string()], start, 12))
                        .map_or(fallback, |(_, p)| p)
                };
                let _ = tx.send(Some(p));
            });
        }
        trips.insert(
            key,
            Trip {
                rune,
                what,
                enhanced,
                from,
                started: Instant::now(),
                last_tick: Instant::now(),
                next_try: Instant::now(),
                anchor: None,
                retries: 0,
                ready: rx,
                target: None,
                route,
                source,
                cached,
            },
        );
        Ok(())
    }
    pub(super) fn sc_rune_tick(&mut self) {
        if !enabled() || self.remote.is_some() || self.player.is_none() {
            return;
        }
        let who = self.native_acting_id();
        let key = (self.sc_rune_dir(), who);
        let now = Instant::now();
        let initial = SESSIONS.with_borrow_mut(|sessions| {
            if sessions.len() > 256 {
                sessions.retain(|_, (_, t)| now.duration_since(*t) < Duration::from_secs(300));
            }
            let visit = sessions
                .entry(key.clone())
                .or_insert((None, now - Duration::from_secs(120)));
            let first =
                visit.0 != self.lan_acting || now.duration_since(visit.1) > Duration::from_secs(60);
            *visit = (self.lan_acting, now);
            first
        });
        if initial && let Ok(r) = self.sc_research() {
            WATCHES.with_borrow_mut(|w| {
                w.remove(&key);
            });
            self.sc_send_research(&r, None);
        }
        self.sc_build_tick();
        self.sc_check_field();
        self.sc_watch_glyph(&key, now);
        let Some(mut trip) = TRIPS
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .remove(&key)
        else {
            return;
        };
        if self
            .player
            .as_ref()
            .is_none_or(|p| p.health.dead_or_dying())
            || trip.from.dim != self.dim.id()
        {
            return;
        }
        trip.last_tick = now;
        if now < trip.next_try {
            TRIPS.get().unwrap().lock().unwrap().insert(key, trip);
            return;
        }
        if trip.target.is_none() {
            trip.target = trip.ready.try_recv().ok().flatten();
        }
        if trip.started.elapsed() > Duration::from_secs(20) {
            self.sc_retry_landing(&mut trip);
        }
        if let Some([x, _, z]) = trip.target {
            if !self
                .border
                .geometry()
                .is_within_bounds(x as f64 + 0.5, z as f64 + 0.5, 0.)
            {
                self.sc_retry_landing(&mut trip);
                TRIPS.get().unwrap().lock().unwrap().insert(key, trip);
                return;
            }
            self.place_portal_ticket((x >> 4, z >> 4));
            if self.world.chunks.contains_key(&(x >> 4, z >> 4)) {
                let mut landing = None;
                // Inspect the linked X/Z first. Search neighbours only if the surface is unsafe.
                let offsets = [0i32, -1, 1, -2, 2, -3, 3];
                for dx in offsets {
                    for dz in offsets {
                        let distance = dx * dx + dz * dz;
                        if landing.is_some_and(|(best, _)| best <= distance) {
                            continue;
                        }
                        let xx = x + dx;
                        let zz = z + dz;
                        if !self.border.geometry().is_within_bounds(
                            xx as f64 + 0.5,
                            zz as f64 + 0.5,
                            0.,
                        ) {
                            continue;
                        }
                        if let Some(p) = self.sc_top_landing(xx, zz)
                            && (trip.what != Outcome::Stray
                                || (0..3)
                                    .map(|i| i64::from(p[i]) - i64::from(trip.from.pos[i]))
                                    .map(|d| d * d)
                                    .sum::<i64>()
                                    <= 400 * 400)
                        {
                            landing = Some((distance, p));
                        }
                    }
                }
                let Some((_, p)) = landing else {
                    self.sc_retry_landing(&mut trip);
                    TRIPS.get().unwrap().lock().unwrap().insert(key, trip);
                    return;
                };
                // Both preview and travel resolve through the same per-pillar file.
                // If another player settled this route while we waited for generation,
                // validate their published column before exposing a different landing.
                if let Some(path) = &trip.route {
                    match read::<Pos>(path) {
                        Ok(latest)
                            if latest != trip.cached
                                && latest.is_some_and(|q| q[0] != p[0] || q[2] != p[2]) =>
                        {
                            trip.cached = latest;
                            trip.target = latest;
                            trip.anchor = None;
                            trip.started = now;
                            TRIPS.get().unwrap().lock().unwrap().insert(key, trip);
                            return;
                        }
                        Ok(latest) => {
                            if latest.is_some_and(|old| old != p) {
                                self.sc_surveys(|s| {
                                    s.forget(&trip.source, trip.rune)
                                });
                            }
                            if latest != Some(p)
                                && let Err(e) = save(path, &p)
                            {
                                log::error!("Could not retain rune landing: {e}");
                                if trip.what != Outcome::Survey {
                                    self.sc_notice("The gateway could not save its destination.");
                                }
                                return;
                            }
                        }
                        Err(e) => {
                            log::error!("Could not read rune landing: {e}");
                            return;
                        }
                    }
                }
                if trip.what == Outcome::Survey {
                    if let Some(biome) = self.biome_name_at_block(p) {
                        let reply = SurveyResult {
                            source: trip.source,
                            glyph: trip.rune,
                            landing: p,
                            biome,
                        };
                        self.sc_surveys(|s| s.remember(reply.clone()));
                        self.sc_send_survey(&reply);
                        WATCHES.with_borrow_mut(|w| {
                            if let Some(v) = w.get_mut(&key) {
                                v.sent = Some((reply.source, reply.glyph));
                            }
                        });
                    }
                    return;
                }
                let dest = Site::new(self.dim.id(), p);
                let Ok(mut r) = self.sc_research() else {
                    self.sc_notice("Your rune research could not be read.");
                    return;
                };
                if !self.sc_go(&dest) {
                    self.sc_retry_landing(&mut trip);
                    TRIPS.get().unwrap().lock().unwrap().insert(key, trip);
                    return;
                }
                let mut learned = 0;
                if matches!(trip.what, Outcome::Learn | Outcome::Eureka) {
                    learned += usize::from(r.learn(trip.rune));
                    if trip.what == Outcome::Eureka && learned == 1 {
                        let unknown: Vec<_> =
                            (0..26u8).filter(|n| !r.learned.contains(n)).collect();
                        if !unknown.is_empty() {
                            let n = self
                                .world
                                .random
                                .next_int_bound(unknown.len() as i32)
                                .unwrap() as usize;
                            learned += usize::from(r.learn(unknown[n]));
                        }
                    }
                }
                r.log(trip.from, dest, if trip.what == Outcome::Stray { TripIcon::Mishap } else { TripIcon::Carving(trip.rune) });
                if self.sc_store_research(&r) {
                    self.sc_notice(match (trip.what, learned) {
                        (Outcome::Stray, _) => "that didn't go according to plan!",
                        (Outcome::Known, _) => "The rune brought you to its destination.",
                        (Outcome::Travel, _) => "that worked,  but I didn't learn anything",
                        (_, 2) => "eureka! I learned two runes!",
                        (_, 1) if trip.enhanced => "success! I learned a new rune.",
                        (_, 1) => "success! I learned a new rune!",
                        _ => "that worked, but I already knew this rune.",
                    });
                }
                return;
            }
        }
        TRIPS.get().unwrap().lock().unwrap().insert(key, trip);
    }
    // Keep the original effect and discovery roll. Each retry tickets one
    // candidate; the next player tick handles it after the chunk is available.
    fn sc_retry_landing(&mut self, trip: &mut Trip) {
        let origin = trip.from.pos;
        let anchor = *trip.anchor.get_or_insert(trip.target.unwrap_or(origin));
        trip.retries = trip.retries.saturating_add(1);
        let (center, radius) = if trip.what == Outcome::Stray {
            (origin, 395.)
        } else if trip.retries % 8 == 0 {
            // Even a void structure or a far-away border can have no safe ground.
            (origin, 16.)
        } else {
            (anchor, f64::from(trip.retries.min(16)) * 24.)
        };
        let angle = self.world.random.next_double() * std::f64::consts::TAU;
        let distance = self.world.random.next_double().sqrt() * radius;
        let [xmin, zmin, xmax, zmax] = self.border.geometry().bounds(0.);
        let inside = |v: f64, lo: f64, hi: f64| v.max(lo.ceil()).min((hi - 1.).floor()) as i32;
        trip.target = Some([
            inside(center[0] as f64 + distance * angle.cos(), xmin, xmax),
            center[1],
            inside(center[2] as f64 + distance * angle.sin(), zmin, zmax),
        ]);
        trip.started = Instant::now();
        trip.next_try = trip.started + Duration::from_millis(250);
    }
    fn sc_go(&mut self, d: &Site) -> bool {
        if !vanilla_dimension(self.dim.id()) || !vanilla_dimension(&d.dim) {
            return false;
        }
        let who = self.native_acting_id();
        self.native_teleport_player(
            who,
            &d.dim,
            [
                d.pos[0] as f64 + 0.5,
                d.pos[1] as f64,
                d.pos[2] as f64 + 0.5,
            ],
            self.yaw,
            self.pitch,
        )
    }
    // All mosaics are numbered in world order (even when read from behind).
    // Older fields have one repeated block ID: walk to their lower left once.
    fn sc_field_center(&self, mut p: Pos, name: &str, axis: usize) -> Pos {
        if let Some(tile) = name.rsplit('_').nth(1).and_then(|s| s.parse::<i32>().ok()).filter(|t| (0..9).contains(t)) {
            p[axis] += 1 - tile % 3;
            p[1] += tile / 3 - 1;
        } else {
            for i in [axis, 1] {
                for _ in 0..2 {
                    let mut neighbor = p;
                    neighbor[i] -= 1;
                    if self.block_name(neighbor).as_ref() != name { break; }
                    p = neighbor;
                }
                p[i] += 1;
            }
        }
        p
    }
    fn sc_looked_at_rune(&self) -> Option<(Site, u8)> {
        let eye = self.eye_position()?;
        let dir = super::view_vector(self.yaw, self.pitch);
        let solid = self.clip_ray(eye, dir, 5.);
        let limit = solid.as_ref().map_or(5., |h| {
            (0..3)
                .map(|i| (h.point[i] - eye[i]).powi(2))
                .sum::<f64>()
                .sqrt()
        });
        // Portal fields have no pickable outline. Intersect their thin center plane,
        // stopping at the first solid hit so labels cannot be read through walls.
        let mut previous = [i32::MIN; 3];
        let mut t = 0.;
        while t <= limit + 0.065 {
            let point: Pos =
                std::array::from_fn(|i| (eye[i] + dir[i] * t.min(limit)).floor() as i32);
            if point != previous {
                previous = point;
                let name = self.block_name(point);
                if let Some(axis) = field_axis(&name) {
                    let normal = 2 - axis;
                    if dir[normal].abs() > 1e-8 {
                        let cross = (point[normal] as f64 + 0.5 - eye[normal]) / dir[normal];
                        if (0.0..=limit + 0.001).contains(&cross)
                            && [axis, 1]
                                .iter()
                                .all(|&i| (eye[i] + dir[i] * cross).floor() as i32 == point[i])
                        {
                            return Some((
                                Site::new(self.dim.id(), self.sc_field_center(point, &name, axis)),
                                if name.contains("_unknown_") { 27 } else { 26 },
                            ));
                        }
                    }
                }
            }
            t += 0.0625;
        }
        let hit = solid?;
        let name = self.block_name(hit.pos);
        // A pickable field surface is 1/8 block nearer than its center plane.
        // A closer solid hit has already cut off the ray above.
        if let Some(axis) = field_axis(&name) {
            let site = Site::new(self.dim.id(), self.sc_field_center(hit.pos, &name, axis));
            return Some((site, if name.contains("_unknown_") { 27 } else { 26 }));
        }
        Some((Site::new(self.dim.id(), hit.pos), glyph(&name, "strangely_charming:runestone_")?))
    }
    fn sc_surveys<T>(&self, apply: impl FnOnce(&mut Surveys) -> T) -> T {
        SURVEYS.with_borrow_mut(|s| {
            if !s
                .scope
                .as_ref()
                .is_some_and(|(root, seed)| root == &self.world_root && *seed == self.level.seed)
            {
                s.scope = Some((self.world_root.clone(), self.level.seed));
                s.results = Surveys::default();
            }
            apply(&mut s.results)
        })
    }
    fn sc_forget_frame_survey(&mut self, f: &Frame) {
        let site = Site::new(self.dim.id(), f.at(0, 2));
        self.sc_surveys(|s| s.forget(&site, 26));
        forget_client_frame(&site);
        let root = self.sc_rune_dir();
        let recipients = WATCHES.with_borrow_mut(|w| {
            let mut ids = Vec::new();
            for ((world, id), watch) in w.iter_mut() {
                if world != &root { continue; }
                if watch.sent.as_ref().is_some_and(|(s,k)| *k >= 26 && s == &site) { watch.sent = None; }
                // Only viewers who were sent this frame can have it cached.
                if let Some(i) = watch.frames.iter().position(|s| s == &site) {
                    watch.frames.remove(i);
                    ids.push(*id);
                }
            }
            ids
        });
        if !recipients.is_empty() {
            let data = serde_json::to_vec(&site).unwrap();
            for id in recipients {
                if !self.native_send(id, Clientbound::CustomPayload(CustomPayload {
                    channel: FORGET_FRAME.into(), data: data.clone(),
                })) {
                    WATCHES.with_borrow_mut(|w| {
                        if let Some(v) = w.get_mut(&(root.clone(),id)) { v.invalidated.push(site.clone()); }
                    });
                }
            }
        }
    }
    fn sc_watch_glyph(&mut self, key: &(PathBuf, u128), now: Instant) {
        // Here we run in the viewer's own player tick, even when the LAN roster
        // was temporarily unavailable in a different player's block callback.
        let invalidated = WATCHES.with_borrow_mut(|w| w.get_mut(key).map(|v| std::mem::take(&mut v.invalidated)));
        for site in invalidated.into_iter().flatten() {
            if self.lan_acting.is_some() {
                self.native_send(key.1, Clientbound::CustomPayload(CustomPayload {
                    channel: FORGET_FRAME.into(), data:serde_json::to_vec(&site).unwrap(),
                }));
            }
        }
        let mask = WATCHES.with_borrow_mut(|w| {
            if w.len() > 256 {
                w.retain(|owner, v| {
                    owner == key
                        || v.poll
                            .is_some_and(|t| now.duration_since(t) < Duration::from_secs(300))
                });
            }
            let watch = w.get_mut(key)?;
            if watch
                .poll
                .is_some_and(|t| now.duration_since(t) < Duration::from_millis(250))
            {
                return None;
            }
            watch.poll = Some(now);
            Some(watch.mask)
        });
        let Some(mask) = mask else {
            return;
        };
        let target = self
            .sc_looked_at_rune()
            .filter(|(_, id)| *id >= 26 || mask & (1 << id) != 0);
        if let Some((site, rune)) = &target {
            let done = WATCHES.with_borrow(|w| {
                w.get(key)
                    .is_some_and(|a| a.sent.as_ref() == target.as_ref() && a.retry.is_none_or(|retry| now < retry))
            });
            if !done {
                if let Some(reply) = self.sc_surveys(|s| s.get(site, *rune).cloned()) {
                    self.sc_send_survey(&reply);
                    WATCHES.with_borrow_mut(|w| {
                        let w = w.get_mut(key).unwrap();
                        w.sent = target.clone();
                        w.retry = reply.biome.is_empty().then(|| now + Duration::from_secs(15));
                    });
                    return;
                }
                if *rune >= 26 {
                    if *rune == 26
                        && let Some(f) = self.sc_field_frame(site.pos)
                        && let Some(ref dest) = f.dest
                    {
                        let biome = self.sc_destination_biome(dest);
                        let reply = SurveyResult {
                            source: site.clone(),
                            glyph: 26,
                            landing: dest.pos,
                            biome: biome.unwrap_or_default(),
                        };
                        self.sc_surveys(|s| s.remember(reply.clone()));
                        self.sc_send_survey(&reply);
                        WATCHES.with_borrow_mut(|w| w.get_mut(key).unwrap().retry =
                            reply.biome.is_empty().then(|| now + Duration::from_secs(15)));
                    }
                    WATCHES.with_borrow_mut(|w| {
                        if let Some(a) = w.get_mut(key) {
                            a.sent = target.clone();
                        }
                    });
                    return;
                }
                let _ = self.sc_start_trip(*rune, Outcome::Survey, site.clone(), site.pos, false);
            }
        } else {
            WATCHES.with_borrow_mut(|w| {
                if let Some(v) = w.get_mut(key) {
                    v.sent = None;
                }
            });
            if let Some(trips) = TRIPS.get() {
                let mut trips = trips.lock().unwrap();
                if trips.get(key).is_some_and(|t| t.what == Outcome::Survey) {
                    trips.remove(key);
                }
            }
        }
    }
    fn sc_send_survey(&mut self, reply: &SurveyResult) {
        if reply.glyph == 26 {
            let key = (self.sc_rune_dir(), self.native_acting_id());
            WATCHES.with_borrow_mut(|w| {
                let frames = &mut w.entry(key).or_default().frames;
                frames.retain(|s| s != &reply.source);
                frames.push_front(reply.source.clone());
                frames.truncate(SURVEY_LIMIT);
            });
        }
        let data = serde_json::to_vec(reply).unwrap();
        if self.lan_acting.is_some() {
            let who = self.native_acting_id();
            self.native_send(
                who,
                Clientbound::CustomPayload(CustomPayload {
                    channel: SURVEY.into(),
                    data,
                }),
            );
        } else {
            receive_survey(&data);
        }
    }
    pub(super) fn sc_rune_label(&self, b: &mut HudBatch) {
        if !enabled() || self.screen != super::Screen::Playing {
            return;
        }
        let looking = self
            .sc_looked_at_rune()
            .filter(|(_, id)| *id >= 26 || sc_glyphs::client_mask() & (1 << id) != 0);
        HINT.with_borrow_mut(|h| {
            let now = Instant::now();
            let looking = looking.map(|(site,id)| {
                let name = match id {
                    27 => "???".into(),
                    26 => "Rune Portal".into(),
                    _ => CLIENT.with_borrow(|r| r.name(id).to_owned()),
                };
                let detail = h.results.get(&site,id).map(|r| {
                    let biome = if r.biome.is_empty() { String::new() } else { friendly_biome(&r.biome) };
                    let coords = if id == 26 {
                        format!("X {}  Y {}  Z {}",r.landing[0],r.landing[1],r.landing[2])
                    } else { format!("X {}  Z {}",r.landing[0],r.landing[2]) };
                    (biome, coords)
                });
                ((site,id),name,detail)
            });
            h.update(now, self.dim.id(), looking);
            let Some(label) = &h.last else { return };
            let opacity = label.opacity(now);
            let atlas = &self.gui.atlas;
            let cx = b.width * 0.5;
            let mut y = (b.height * 0.5 + 32.).round();
            let (biome, coords, detail_alpha) = label.detail.as_ref().map_or(("", "", 0.), |(biome, coords, since)|
                (biome.as_str(), coords.as_str(), (now.duration_since(*since).as_secs_f32()/FADE_IN).min(1.)));
            for (i, part) in [label.title.as_str(), biome, coords].into_iter().enumerate() {
                let scale = if i == 0 { 1.25 } else { 1. };
                let opacity = if i == 0 { opacity } else { opacity * detail_alpha };
                if part.is_empty() { y += 10.; continue; }
                for line in
                    horizon_client::multiplayer::wrap(part, (b.width - 16.).max(50.) / scale, atlas)
                {
                    let start = b.vertices.len();
                    b.text(
                        atlas,
                        &line,
                        (cx - HudBatch::text_width(atlas, &line) * 0.5).round(),
                        y,
                        rgba(255, 255, 85, (opacity * 255.) as u8),
                        true,
                    );
                    b.scale_since(start, cx, y, scale, scale);
                    y += if i == 0 { 14. } else { 10. };
                }
            }
        });
    }
}
#[derive(Default)]
struct Watch {
    mask: u32,
    poll: Option<Instant>,
    sent: Option<(Site, u8)>,
    retry: Option<Instant>,
    frames: VecDeque<Site>,
    invalidated: Vec<Site>,
}
thread_local! {static WATCHES:RefCell<HashMap<(PathBuf,u128),Watch>>=RefCell::default();static HINT:RefCell<Hint>=RefCell::default();}
pub(super) const SURVEY: &str = "strangely_charming:rune_survey";
#[derive(Clone, Serialize, Deserialize)]
struct SurveyResult {
    source: Site,
    glyph: u8,
    landing: Pos,
    biome: String,
}
const SURVEY_LIMIT: usize = 512;
#[derive(Default)]
struct Surveys(VecDeque<(SurveyResult, Instant)>);
impl Surveys {
    fn get(&self, site: &Site, glyph: u8) -> Option<&SurveyResult> {
        self.0.iter().find_map(|(r, at)| (&r.source == site && r.glyph == glyph
            && (!r.biome.is_empty() || at.elapsed() < Duration::from_secs(15))).then_some(r))
    }
    fn forget(&mut self, site: &Site, glyph: u8) {
        self.0.retain(|(r,_)| &r.source != site || r.glyph != glyph);
    }
    fn remember(&mut self, result: SurveyResult) {
        self.forget(&result.source,result.glyph);
        self.0.push_front((result,Instant::now()));
        self.0.truncate(SURVEY_LIMIT);
    }
}
#[derive(Default)]
struct WorldSurveys {
    scope: Option<(PathBuf, i64)>,
    results: Surveys,
}
thread_local! {static SURVEYS: RefCell<WorldSurveys> = RefCell::default();}
const FADE_IN: f32 = 0.24;
const FADE_OUT: f32 = 0.8;
struct Label {
    key: (Site,u8),
    title: String,
    detail: Option<(String,String,Instant)>,
    transition: Instant,
    from: f32,
    visible: bool,
}
impl Label {
    fn opacity(&self, now: Instant) -> f32 {
        let secs = now.duration_since(self.transition).as_secs_f32();
        (self.from + if self.visible { secs / FADE_IN } else { -secs / FADE_OUT }).clamp(0.,1.)
    }
    fn visible(&mut self, now: Instant, show: bool) {
        if self.visible != show {
            self.from = self.opacity(now);
            self.transition = now;
            self.visible = show;
        }
    }
}
#[derive(Default)]
struct Hint {
    results: Surveys,
    last: Option<Label>,
}
impl Hint {
    fn update(&mut self, now: Instant, dim: &str, hit: Option<((Site,u8),String,Option<(String,String)>)>) {
        if let Some((key,title,detail)) = hit {
            if self.last.as_ref().is_none_or(|l| l.key != key) {
                self.last = Some(Label {key, title:title.clone(), detail:None, transition:now, from:0., visible:true});
            }
            let l = self.last.as_mut().unwrap();
            l.visible(now,true);
            l.title = title;
            if let Some((biome,coords)) = detail {
                if l.detail.as_ref().is_none_or(|(b,c,_)| (b,c)!=(&biome,&coords)) { l.detail=Some((biome,coords,now)); }
            } else { l.detail = None; }
        } else if let Some(l) = &mut self.last {
            l.visible(now,false);
            if l.key.0.dim != dim || l.opacity(now)<=0. { self.last=None; }
        }
    }
}
pub(super) fn receive_survey(data: &[u8]) {
    if data.len() <= 2048 && let Ok(r) = serde_json::from_slice::<SurveyResult>(data) && r.glyph <= 26 {
        HINT.with_borrow_mut(|s| s.results.remember(r));
    }
}
pub(super) const FORGET_FRAME: &str = "strangely_charming:forget_frame";
fn forget_client_frame(site: &Site) {
    HINT.with_borrow_mut(|s| {
        s.results.forget(site,26);
        if let Some(label) = &mut s.last && label.key.1 >= 26 && &label.key.0 == site { label.detail=None; }
    });
}
pub(super) fn receive_forget_frame(data: &[u8]) {
    if data.len() <= 512 && let Ok(site) = serde_json::from_slice::<Site>(data) { forget_client_frame(&site); }
}
fn friendly_biome(id: &str) -> String {
    let (ns, path) = id.split_once(':').unwrap_or(("minecraft", id));
    if let Some(name) = horizon_client::lang::current().and_then(|l| {
        l.get(&format!("biome.{ns}.{}", path.replace('/', ".")))
            .map(str::to_owned)
    }) {
        return name;
    }
    path.split('_')
        .map(|part| {
            let mut c = part.chars();
            c.next()
                .map(|x| x.to_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PortalTheme {
    #[default]
    Ordinary,
    Jungle,
    Unknown,
    Nether,
    Painted(String),
}

#[derive(Serialize, Deserialize)]
struct Frame {
    #[serde(default)]
    sided_runes: bool,
    // Frames lit before 0.2.30 contain the original plain field blocks.
    #[serde(default)]
    theme: PortalTheme,
    bottom: Pos,
    axis: usize,
    sign: i32,
    address: Address,
    dest: Option<Site>,
}
pub(super) const FRAME_GLYPHS: [(i32, i32); 12] = [
    (-2, 1),
    (-2, 2),
    (-2, 3),
    (-1, 4),
    (0, 4),
    (1, 4),
    (2, 3),
    (2, 2),
    (2, 1),
    (1, 0),
    (0, 0),
    (-1, 0),
];
impl Frame {
    fn at(&self, h: i32, y: i32) -> Pos {
        let mut p = self.bottom;
        p[self.axis] += self.sign * h;
        p[1] += y;
        p
    }
    fn field(&self) -> impl Iterator<Item = Pos> + '_ {
        (-1..=1).flat_map(move |x| (1..=3).map(move |y| self.at(x, y)))
    }
    fn scan(&self, g: &Game<'_>) -> Option<Address> {
        let mut a = [0; 12];
        for (i, (h, y)) in FRAME_GLYPHS.into_iter().enumerate() {
            a[i] = glyph(
                &g.block_name(self.at(h, y)),
                "strangely_charming:runic_amethyst_",
            )?
        }
        Some(a)
    }
    fn field_block(&self, p: Pos) -> String {
        let axis = if self.axis == 0 { "x" } else { "z" };
        match &self.theme {
            PortalTheme::Ordinary => format!("{FIELD_PREFIX}{axis}"),
            PortalTheme::Nether => format!("{FIELD_PREFIX}nether_{axis}"),
            painting => {
                let theme = match painting {
                    PortalTheme::Jungle => "jungle",
                    PortalTheme::Painted(name) => name,
                    _ => "unknown",
                };
                let col = 1 + p[self.axis] - self.bottom[self.axis];
                let row = self.bottom[1] + 3 - p[1];
                format!("{FIELD_PREFIX}{theme}_{}_{axis}", row * 3 + col)
            }
        }
    }
}
const FRAME_PREFIX: &str = "strangely_charming:runic_amethyst_";
const FIELD_PREFIX: &str = "strangely_charming:rune_portal_";
fn field_axis(id: &str) -> Option<usize> {
    let id = id.strip_prefix(FIELD_PREFIX)?;
    match id.rsplit('_').next()? {
        "x" => Some(0),
        "z" => Some(2),
        _ => None,
    }
}
type Instance = (i64, String);
static CHANGES: OnceLock<Mutex<HashMap<Instance, Vec<Pos>>>> = OnceLock::new();
// A packet callback can temporarily borrow the LAN roster out of Game. Keep
// the last player-tick positions as well, so lighting around a bystander never
// counts as entry even in that callback.
thread_local! {static FEET:RefCell<HashMap<(PathBuf,u128),(Site,Instant)>> = RefCell::default();}
static INSIDE: OnceLock<Mutex<HashMap<(PathBuf, u128), Instant>>> = OnceLock::new();
pub(super) fn frame_change(seed: i64, dim: &str, p: Pos) {
    if enabled() {
        CHANGES
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .entry((seed, dim.into()))
            .or_default()
            .push(p);
    }
}
impl Game<'_> {
    fn sc_frame_path(&self, p: Pos) -> PathBuf {
        self.sc_rune_dir()
            .join("frames")
            .join(file_key(&Site::new(self.dim.id(), p)))
    }
    fn sc_destination_biome(&self, site: &Site) -> Option<String> {
        use horizon_client::save::{SaveDir, dimension_dir};
        if !vanilla_dimension(&site.dim) {
            return None;
        }
        let dim = super::Dim::from_id(&site.dim);
        let [x, y, z] = site.pos;
        let key = (x >> 4, z >> 4);
        let biomes = world_biomes!(self);
        let world = if self.dim == dim {
            Some(&self.world)
        } else {
            self.levels.parked_world(dim)
        };
        let id = world
            .and_then(|w| w.chunks.get(&key))
            .map(|c| c.biome(x, y, z))
            .or_else(|| {
                // Sample only an already-saved chunk, once on ignition. Never generate or ticket terrain for artwork.
                let other;
                let save = if self.dim == dim {
                    &*self.save
                } else {
                    let root = dimension_dir(&self.world_root, dim.id());
                    if !root
                        .join("region")
                        .join(format!("r.{}.{}.mca", key.0 >> 5, key.1 >> 5))
                        .is_file()
                    {
                        return None;
                    }
                    other = SaveDir::new(root, dim.shape());
                    &other
                };
                save.load(key.0, key.1, self.pack.assets.registry, biomes)
                    .map(|c| c.biome(x, y, z))
            });
        id.map(|id| biomes.get(id).name.to_string())
    }
    fn sc_portal_theme(&self, site: &Site) -> PortalTheme {
        let theme = self.sc_destination_biome(site).as_deref().map_or(
            "generic",
            horizon_mods::strangely_charming_content::portal_painting,
        );
        match (theme, site.dim.as_str()) {
            ("generic", "minecraft:the_end") => PortalTheme::Painted("end_islands".into()),
            ("generic", "minecraft:the_nether") => PortalTheme::Painted("nether_wastes".into()),
            ("jungle", _) => PortalTheme::Jungle,
            (name, _) => PortalTheme::Painted(name.into()),
        }
    }
    pub(super) fn sc_igniter_target(
        &self,
        p: Pos,
        held: Option<&horizon_client::items::ItemStack>,
    ) -> bool {
        enabled()
            && horizon_mods::evanders_config::enabled("knowledge_stone")
            && held.is_some_and(|s| s.item.as_ref() == super::sc_field_items::KNOWLEDGE)
            && {
                let name = self.block_name(p);
                name.starts_with(FRAME_PREFIX) || field_axis(&name).is_some()
            }
    }
    pub(super) fn sc_ignite(&mut self, p: Pos) -> bool {
        if !self.sc_igniter_target(p, self.held()) {
            return false;
        }
        if self.remote.is_some() || field_axis(&self.block_name(p)).is_some() {
            return true;
        }
        let registry = self.sc_rune_dir().join("addresses");
        let mut unknown = Vec::new();
        let mut selected = None;
        for axis in [0, 2] {
            let mut frame = Frame {
                sided_runes: true,
                theme: PortalTheme::Ordinary,
                bottom: p,
                axis,
                sign: 1,
                address: [0; 12],
                dest: None,
            };
            let Some(front) = frame.scan(self) else {
                continue;
            };

            let back = reflected(front);
            let readings = (|| -> io::Result<_> {
                let first = read::<Site>(&registry.join(code(front)))?;
                let second = if front == back {
                    None
                } else {
                    read::<Site>(&registry.join(code(back)))?
                };
                Ok((first, second))
            })();
            let (address, sign, destination) = match readings {
                Err(err) => {
                    log::error!("Could not read the portal address registry: {err}");
                    self.sc_notice("The portal could not read its destination.");
                    return true;
                }
                Ok((None, None)) => {
                    frame.address = front;
                    frame.theme = PortalTheme::Unknown;
                    unknown.push(frame);
                    continue;
                }
                Ok((Some(dest), None)) => (front, 1, dest),
                Ok((None, Some(dest))) => (back, -1, dest),
                Ok((Some(first), Some(second))) if first == second => (front, 1, first),
                Ok((Some(first), Some(second))) => {
                    // 0.2.26 could register both sides for different places. Horizontal
                    // positive runs to the viewer's right from south (+X) or west (+Z).
                    let Some(pos) = self.player.as_ref().map(|p| p.position) else {
                        return true;
                    };
                    let offset = if axis == 0 {
                        pos.z - (f64::from(p[2]) + 0.5)
                    } else {
                        (f64::from(p[0]) + 0.5) - pos.x
                    };
                    if offset.abs() <= 0.5 {
                        self.sc_notice("Two destinations. Stand in front of the glyphs you want and use the stone again.");
                        return true;
                    }
                    if offset > 0. {
                        (front, 1, first)
                    } else {
                        (back, -1, second)
                    }
                }
            };
            if !vanilla_dimension(self.dim.id()) || !vanilla_dimension(&destination.dim) {
                self.sc_notice("This portal leads beyond the three known dimensions.");
                return true;
            }
            frame.address = address;
            frame.sign = sign;
            frame.theme = self.sc_portal_theme(&destination);
            frame.dest = Some(destination);
            if selected.replace(frame).is_some() {
                self.sc_notice("Two frames share this center block. Give each frame its own base.");
                return true;
            }
        }
        let frame = match selected {
            Some(known) => known,
            None if unknown.len() == 1 && vanilla_dimension(self.dim.id()) => {
                unknown.pop().unwrap()
            }
            None => {
                self.sc_notice(if unknown.len() > 1 {
                    "Two frames share this center block. Give each frame its own base."
                } else {
                    "Use the middle of a complete runic amethyst frame."
                });
                return true;
            }
        };
        if !frame.field().all(|cell| {
            self.res
                .facts()
                .is_air(self.world.block(cell[0], cell[1], cell[2]))
                || field_axis(&self.block_name(cell)) == Some(frame.axis)
        }) {
            self.sc_notice("Clear the opening before lighting the portal.");
            return true;
        }
        if save(&self.sc_frame_path(p), &frame).is_err() {
            self.sc_notice("The portal could not save its destination.");
            return true;
        }
        self.sc_forget_frame_survey(&frame);
        for cell in frame.field() {
            if !self.sc_set_rune_block(cell, &frame.field_block(cell)) {
                self.sc_remove_frame(&frame);
                self.sc_notice("The portal could not open.");
                return true;
            }
        }
        self.play_at_feet("minecraft:block.end_portal.spawn");
        // Anyone standing in the opening as it appears must step out and enter it.
        let matches =
            |dim: &str, pos: Pos| dim == self.dim.id() && frame.field().any(|cell| cell == pos);
        let mut occupied: Vec<_> = self
            .native_server_players()
            .into_iter()
            .filter(|p| matches(&p.dimension, p.position.map(|n| n.floor() as i32)))
            .map(|p| p.uuid)
            .collect();
        FEET.with_borrow(|seen| {
            occupied.extend(
                seen.iter()
                    .filter(|((root, _), (site, time))| {
                        root == &self.world_root
                            && time.elapsed() < Duration::from_secs(2)
                            && matches(&site.dim, site.pos)
                    })
                    .map(|((_, id), _)| *id),
            );
        });
        if self.player.as_ref().is_some_and(|p| {
            matches(
                self.dim.id(),
                [p.position.x, p.position.y, p.position.z].map(|n| n.floor() as i32),
            )
        }) {
            occupied.push(self.native_acting_id());
        }
        let mut inside = INSIDE.get_or_init(Default::default).lock().unwrap();
        for id in occupied {
            inside.insert((self.world_root.clone(), id), Instant::now());
        }
        drop(inside);
        if let Some(dest) = &frame.dest {
            let [x, y, z] = dest.pos;
            self.sc_notice(format!(
                "Portal to {} ({x}, {y}, {z})",
                super::sc_travel::dimension_name(&dest.dim)
            ));
        }
        true
    }
    fn sc_set_rune_block(&mut self, p: Pos, id: &str) -> bool {
        self.sc_grave_set(p, id)
    }
    fn sc_close_frame(&mut self, f: &Frame) {
        self.sc_remove_frame(f);
        let [x, y, z] = f.bottom.map(f64::from);
        self.native_play_sound_in_place(
            "minecraft:block.glass.break",
            [x + 0.5, y + 2.5, z + 0.5],
            4,
            1.0,
            1.0,
        );
    }
    fn sc_remove_frame(&mut self, f: &Frame) {
        self.sc_forget_frame_survey(f);
        let _ = fs::remove_file(self.sc_frame_path(f.bottom));
        for p in f.field() {
            if field_axis(&self.block_name(p)) == Some(f.axis) {
                self.sc_set_rune_block(p, "minecraft:air");
            }
        }
    }
    pub(super) fn sc_inscribe_frame_rune(
        &mut self,
        p: Pos,
        glyph: u8,
        axis: usize,
        sign: i32,
    ) -> bool {
        // The builder enumerates the glyphs from the perspective of the person placing it.
        let facing = match (axis, sign) {
            (0, 1) => "south",
            (0, -1) => "north",
            (2, 1) => "west",
            (2, -1) => "east",
            _ => return false,
        };
        let id = format!("{FRAME_PREFIX}{glyph}");
        let Some(state) = self
            .pack
            .assets
            .registry
            .default_state(&id)
            .and_then(|s| self.with_props(s, &[("facing", facing)]))
        else {
            return false;
        };
        let [x, y, z] = p;
        self.world.block(x, y, z) == state || self.native_set_block(self.dim.id(), p, state, 3)
    }
    fn sc_field_frame(&mut self, p: Pos) -> Option<Frame> {
        let block = self.block_name(p);
        let axis = field_axis(&block)?;
        for y in 1..=3 {
            for x in -1..=1 {
                let mut base = p;
                base[axis] += x;
                base[1] -= y;
                if !self.block_name(base).starts_with(FRAME_PREFIX) {
                    continue;
                }
                if let Ok(Some(mut f)) = read::<Frame>(&self.sc_frame_path(base))
                    && f.axis == axis
                    && f.field().any(|v| v == p)
                {
                    // Before single-face carvings, lit frames only saved their reading direction.
                    // Upgrade their existing blocks once, when the complete frame is loaded.
                    if !f.sided_runes
                        && f.scan(self) == Some(f.address)
                        && FRAME_GLYPHS
                            .into_iter()
                            .zip(f.address)
                            .all(|((h, y), glyph)| {
                                self.sc_inscribe_frame_rune(f.at(h, y), glyph, f.axis, f.sign)
                            })
                    {
                        f.sided_runes = true;
                        if let Err(err) = save(&self.sc_frame_path(base), &f) {
                            log::warn!("Could not record rune face migration: {err}");
                        }
                    }
                    return Some(f);
                }
            }
        }
        None
    }
    fn sc_check_field(&mut self) {
        let dirty = CHANGES
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .remove(&(self.level.seed, self.dim.id().into()))
            .unwrap_or_default();
        for p in dirty {
            for axis in [0, 2] {
                for y in [0, 1, 2, 3, 4] {
                    for x in -2..=2 {
                        let mut b = p;
                        b[axis] += x;
                        b[1] -= y;
                        if b != p && !self.block_name(b).starts_with(FRAME_PREFIX) {
                            continue;
                        }
                        if let Ok(Some(f)) = read::<Frame>(&self.sc_frame_path(b))
                            && (f.scan(self) != Some(f.address)
                                || !f
                                    .field()
                                    .all(|v| self.block_name(v).as_ref() == f.field_block(v)))
                        {
                            self.sc_close_frame(&f);
                        }
                    }
                }
            }
        }
        let Some(pos) = self.player.as_ref().map(|p| p.position) else {
            return;
        };
        let p = [
            pos.x.floor() as i32,
            pos.y.floor() as i32,
            pos.z.floor() as i32,
        ];
        let key = (self.world_root.clone(), self.native_acting_id());
        FEET.with_borrow_mut(|seen| {
            if seen.len() > 256 {
                seen.retain(|_, (_, t)| t.elapsed() < Duration::from_secs(30));
            }
            seen.insert(key.clone(), (Site::new(self.dim.id(), p), Instant::now()));
        });
        if !self.block_name(p).starts_with(FIELD_PREFIX) {
            if let Some(lock) = INSIDE.get() {
                lock.lock().unwrap().remove(&key);
            }
            return;
        }
        let now = Instant::now();
        let mut lock = INSIDE.get_or_init(Default::default).lock().unwrap();

        if lock.insert(key, now).is_some() {
            return;
        }
        drop(lock);
        let Some(f) = self.sc_field_frame(p) else {
            self.sc_set_rune_block(p, "minecraft:air");
            return;
        };
        if f.scan(self) != Some(f.address) {
            self.sc_close_frame(&f);
            return;
        }
        let mishap = f.dest.is_none() && f.theme == PortalTheme::Unknown;
        let dest = match f.dest {
            Some(d) => d,
            None if f.theme == PortalTheme::Unknown => {
                let mut roll = |n| self.world.random.next_int_bound(n).unwrap();
                Site::new(
                    "minecraft:the_end",
                    [roll(8193) - 4096, 32 + roll(64), roll(8193) - 4096],
                )
            }
            None => return,
        };
        let from = Site::new(self.dim.id(), p);
        let Ok(mut research) = self.sc_research() else {
            self.sc_notice("Could not read your Teleport Log.");
            return;
        };
        if self.sc_go(&dest) {
            research.log(from, dest, if mishap { TripIcon::Mishap } else { TripIcon::Amethyst(f.address[10]) });
            self.sc_store_research(&research);
            self.play_at_feet("minecraft:block.portal.travel");
        }
    }
}
