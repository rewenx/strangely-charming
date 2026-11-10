// The world owns this ledger. A marker is only an address into it; replacing,
// exploding or duplicating a decorative block cannot mint another inventory.
use super::{
    Game,
    sc_grave_hud::{MiningRule, View},
};
use horizon_client::sc_grave_veil as veil;
use horizon_client::{
    item_data::{decode_components, encode_components},
    items::ItemStack,
};
use horizon_core::random::RandomSource;
use horizon_world::{
    nbt::{Tag, parse_snbt},
    snbt_text::to_snbt,
};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
const MODELS: [&str; 3] = [
    "strangely_charming:short_tombstone",
    "strangely_charming:ornate_tombstone",
    "strangely_charming:tall_tombstone",
];
const AIR: &str = "minecraft:air";
type Pos = [i32; 3];
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn death_id(name: &str, t: u64) -> Result<String, String> {
    let mut bytes = [0u8; 12];
    horizon_protocol::crypto::random_bytes(&mut bytes).map_err(|e| e.to_string())?;
    let suffix: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("{name}-{}-{suffix}", date(t)))
}
fn date(t: u64) -> String {
    let z = (t / 86400) as i64 + 719468;
    let era = z / 146097;
    let d = z % 146097;
    let y = (d - d / 1460 + d / 36524 - d / 146096) / 365;
    let a = d - (365 * y + y / 4 - y / 100);
    let m = (5 * a + 2) / 153;
    let day = a - (153 * m + 2) / 5 + 1;
    let month = if m < 10 { m + 3 } else { m - 9 };
    let year = y + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}-{:02}-{:02}Z",
        t / 3600 % 24,
        t / 60 % 60,
        t % 60
    )
}
#[derive(Clone, Serialize, Deserialize)]
struct Slot {
    slot: usize,
    item: String,
    count: u32,
    components: Option<String>,
}
impl Slot {
    fn save(slot: usize, s: &ItemStack) -> Self {
        Self {
            slot,
            item: s.item.to_string(),
            count: s.count,
            components: encode_components(s).map(|c| to_snbt(&c)),
        }
    }
    fn load(&self) -> Result<ItemStack, String> {
        if self.count == 0 || self.item.is_empty() {
            return Err("Invalid saved item".into());
        }
        let components = self
            .components
            .as_deref()
            .map(|c| {
                let (t, n) = parse_snbt(c)?;
                if n != c.len() || !matches!(t, Tag::Compound(_)) {
                    Err("Invalid saved components".to_string())
                } else {
                    Ok(t)
                }
            })
            .transpose()?;
        let (damage, extra) = decode_components(components.as_ref());
        let s = ItemStack {
            item: self.item.as_str().into(),
            count: self.count,
            damage,
            extra,
        };
        if encode_components(&s) != components {
            return Err("This game cannot round-trip the stored item components".into());
        }
        Ok(s)
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Grave {
    id: String,
    owner: String,
    name: String,
    death_utc: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    break_after_seconds: Option<u64>,
    #[serde(default)]
    sequence: u64,
    model: usize,
    dimension: String,
    death: Pos,
    position: Option<Pos>,
    phase: String,
    slots: Vec<Slot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    displaced: Vec<Slot>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Ledger {
    schema: u32,
    recent: Vec<String>,
    records: BTreeMap<String, Grave>,
}
impl Default for Ledger {
    fn default() -> Self {
        Self {
            schema: 1,
            recent: vec![],
            records: BTreeMap::new(),
        }
    }
}
impl Ledger {
    fn latest_active(&self, owner: &str) -> Option<&Grave> {
        let active = |g: &&Grave| g.owner == owner && g.phase == "active" && g.position.is_some();
        self.recent
            .iter()
            .filter_map(|id| self.records.get(id))
            .find(active)
            .or_else(|| {
                self.records
                    .values()
                    .filter(active)
                    .max_by_key(|g| (g.sequence, g.death_utc))
            })
    }
    fn remember(&mut self, mut g: Grave) {
        g.sequence = self
            .records
            .values()
            .map(|g| g.sequence)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        self.recent.insert(0, g.id.clone());
        self.recent.truncate(10);
        self.records.insert(g.id.clone(), g);
        self.prune();
    }
    fn prune(&mut self) {
        self.records
            .retain(|id, g| g.phase != "claimed" || self.recent.contains(id));
    }
}
type Stores = HashMap<PathBuf, Ledger>;
static STORE: OnceLock<Mutex<Stores>> = OnceLock::new();
fn access<T>(dir: &Path, f: impl FnOnce(&Ledger) -> T) -> Result<T, String> {
    let mut s = STORE
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "grave ledger lock")?;
    if !s.contains_key(dir) {
        let path = dir.join("strangely_charming/graves.json");
        let state = match fs::read(&path) {
            Ok(raw) => serde_json::from_slice::<Ledger>(&raw)
                .map_err(|e| format!("{}: {e}", path.display()))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ledger::default(),
            Err(e) => return Err(e.to_string()),
        };
        if state.schema != 1 {
            return Err("Unknown grave ledger version".into());
        }
        s.insert(dir.to_owned(), state);
    }
    Ok(f(&s[dir]))
}
fn change<T>(dir: &Path, f: impl FnOnce(&mut Ledger) -> Result<T, String>) -> Result<T, String> {
    access(dir, |_| ())?;
    let mut all = STORE
        .get()
        .unwrap()
        .lock()
        .map_err(|_| "grave ledger lock")?;
    let mut l = all[dir].clone();
    let out = f(&mut l)?;
    let folder = dir.join("strangely_charming");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let tmp = folder.join("graves.json.new");
    let dst = folder.join("graves.json");
    let save = || -> io::Result<()> {
        let mut w = fs::File::create(&tmp)?;
        serde_json::to_writer_pretty(&mut w, &l)?;
        w.write_all(b"\n")?;
        w.sync_all()?;
        fs::rename(&tmp, &dst)?;
        Ok(())
    };
    save().map_err(|e| format!("{}: {e}", dst.display()))?;
    all.insert(dir.to_owned(), l);
    if let Err(e) = fs::File::open(&folder).and_then(|w| w.sync_all()) {
        log::warn!("Grave directory sync: {e}");
    }
    Ok(out)
}
fn candidates(center: Pos, min: i32, max: i32) -> Vec<Pos> {
    let [cx, cy, cz] = center;
    if min + 3 > max {
        return vec![];
    }
    let y = cy.clamp(min + 1, max - 2);
    let mut v = Vec::with_capacity(4200);
    for dx in -10..=10i32 {
        for dz in -10..=10i32 {
            for dy in -10..=10i32 {
                let ds = dx * dx + dy * dy + dz * dz;
                if ds <= 100 && (min + 1..=max - 2).contains(&(y + dy)) {
                    v.push((ds, dy.abs(), dx, dz, dy));
                }
            }
        }
    }
    v.sort_unstable();
    v.into_iter()
        .map(|(_, _, dx, dz, dy)| [cx + dx, y + dy, cz + dz])
        .collect()
}
pub(super) fn is_marker(name: &str) -> bool {
    MODELS.contains(&name)
}
fn carve(name: &str) -> bool {
    matches!(
        name,
        "minecraft:stone"
            | "minecraft:deepslate"
            | "minecraft:tuff"
            | "minecraft:dirt"
            | "minecraft:grass_block"
            | "minecraft:netherrack"
            | "minecraft:end_stone"
            | "minecraft:andesite"
            | "minecraft:diorite"
            | "minecraft:granite"
            | "minecraft:calcite"
            | "minecraft:basalt"
            | "minecraft:blackstone"
            | "minecraft:cobbled_deepslate"
            | "minecraft:gravel"
            | "minecraft:sand"
    )
}
thread_local! {static DEATH:RefCell<Option<(u128,Vec<(usize,ItemStack)>)>>=const{RefCell::new(None)};}
type GuardKey = (PathBuf, u128);
#[derive(Default)]
struct Shield {
    seen: HashSet<String>,
    wayfinder_visible: bool,
    last: Option<Instant>,
}
static GUARDS: OnceLock<Mutex<HashMap<GuardKey, Shield>>> = OnceLock::new();
// Guest inventory is swapped back into Horizon's hub at the end of its action.
thread_local! {static DIRTY:RefCell<HashMap<(PathBuf,u128),Vec<String>>>=RefCell::new(HashMap::new());}
fn dirty(world: &Path, who: u128, claim: Option<String>) {
    DIRTY.with_borrow_mut(|m| {
        m.entry((world.to_owned(), who)).or_default().extend(claim);
    });
}
impl Game<'_> {
    pub(super) fn sc_checkpoint_graves(&mut self, host: &mut super::lan::LanHost) {
        if DIRTY.with_borrow(|m| m.is_empty()) {
            return;
        }
        use horizon_client::lan::playerdata as pd;
        let root = self.world_root.clone();
        for guest in host
            .hub
            .guests
            .iter_mut()
            .filter(|g| g.placed && !g.released)
        {
            let key = (root.clone(), guest.uuid);
            let Some(ids) = DIRTY.with_borrow_mut(|m| m.remove(&key)) else {
                continue;
            };
            let result = (|| -> Result<(), String> {
                self.finish_player_autosaves(true)
                    .map_err(|e| e.to_string())?;
                self.guest_attachments_into(guest);
                if host.owner.as_ref().is_some_and(|o| o.uuid == guest.uuid) {
                    self.level
                        .player_uuid
                        .get_or_insert(pd::uuid_ints(guest.uuid));
                    pd::store(guest, &mut self.level, None);
                    self.level.write_player(&root).map_err(|e| e.to_string())?;
                }
                pd::save_now(guest, &root)?;
                if !ids.is_empty() {
                    change(&root, |l| {
                        for id in &ids {
                            if let Some(g) = l.records.get_mut(id)
                                && g.phase == "claiming"
                            {
                                g.phase = "claimed".into();
                                g.position = None;
                            }
                        }
                        l.prune();
                        Ok(())
                    })?;
                }
                Ok(())
            })();
            if let Err(e) = result {
                let msg = format!(
                    "Grave checkpoint failed; the ledger is retained for operator inspection: {e}"
                );
                log::error!("{msg}");
                guest.send(&horizon_protocol::packets::play::Clientbound::SystemChat(
                    horizon_protocol::packets::play::SystemChat {
                        content: Tag::String(msg.as_str().into()),
                        overlay: false,
                    },
                ));
            }
        }
    }
    fn sc_slots(&self) -> Vec<(usize, ItemStack)> {
        self.inventory
            .iter()
            .enumerate()
            .chain(std::iter::once((40, &self.offhand)))
            .chain(
                self.armor
                    .iter()
                    .rev()
                    .enumerate()
                    .map(|(i, s)| (36 + i, s)),
            )
            .filter_map(|(i, s)| s.clone().map(|v| (i, v)))
            .collect()
    }
    fn sc_slot_mut(&mut self, i: usize) -> &mut Option<ItemStack> {
        match i {
            36..=39 => &mut self.armor[39 - i],
            40 => &mut self.offhand,
            _ => &mut self.inventory[i],
        }
    }
    pub(super) fn sc_death_start(&mut self, who: u128) {
        veil::stop(who, self.lan_acting);
        if let Some(lock) = GUARDS.get() {
            if let Some(guard) = lock
                .lock()
                .unwrap()
                .get_mut(&(self.world_root.clone(), who))
            {
                guard.last = None;
            }
        }
        DEATH.with(|d| *d.borrow_mut() = None);
        if self.remote.is_some()
            || self.ui.rules.bool("keep_inventory")
            || !horizon_mods::evanders_config::enabled("gravestones")
        {
            return;
        }
        if let Ok(mut guards) = GUARDS.get_or_init(Default::default).lock() {
            guards.remove(&(self.world_root.clone(), who));
        }
        DEATH.with(|d| *d.borrow_mut() = Some((who, self.sc_slots())));
    }
    pub(super) fn sc_capture_drops(
        &mut self,
        ctx: &horizon_client::mod_api::NativeLootDrops<'_>,
        drops: &mut Vec<ItemStack>,
    ) {
        if ctx.entity.map(|e| e.1) != Some("minecraft:player") || !ctx.table.is_empty() {
            return;
        }
        let Some((who, original)) = DEATH.with(|v| v.borrow_mut().take()) else {
            return;
        };
        if ctx.player != Some(who) {
            return;
        }
        let saved = (|| -> Result<String, String> {
            let dir = self.native_world_dir().ok_or("No world directory")?;
            let t = now();
            let name = self.acting_player_name().to_string();
            let id = death_id(&name, t)?;
            let mut used = HashSet::new();
            let mut saved = Vec::new();
            let mut extra = Vec::new();
            for s in drops
                .iter()
                .filter(|s| s.count > 0 && s.item.as_ref() != AIR)
            {
                if let Some((i, _)) = original
                    .iter()
                    .filter(|(i, a)| !used.contains(i) && s.same_item_same_components(a))
                    .min_by_key(|(_, a)| a.count != s.count)
                {
                    used.insert(*i);
                    saved.push(Slot::save(*i, s));
                } else {
                    extra.push(s);
                }
            }
            for s in extra {
                let i = original
                    .iter()
                    .find(|(i, a)| !used.contains(i) && a.item == s.item)
                    .map(|e| e.0)
                    .or_else(|| (0..36).find(|i| !used.contains(i)))
                    .unwrap_or(41 + saved.len());
                used.insert(i);
                saved.push(Slot::save(i, s));
            }
            for s in &saved {
                s.load()?;
            }
            let g = Grave {
                id: id.clone(),
                owner: format!("{who:032x}"),
                name,
                death_utc: t,
                break_after_seconds: None,
                sequence: 0,
                model: self.world.random.next_int_bound(3).unwrap_or(0) as usize,
                dimension: self.dim.id().into(),
                death: ctx.origin.map(|v| v.floor() as i32),
                position: None,
                phase: "pending".into(),
                slots: saved,
                displaced: vec![],
            };
            change(&dir, |l| {
                if l.records.contains_key(&id) {
                    return Err("Grave ID collision".into());
                }
                l.remember(g);
                Ok(())
            })?;
            Ok(id)
        })();
        match saved {
            Err(e) => self.sc_grave_error(&format!(
                "Grave could not be saved; normal death drops apply: {e}"
            )),
            Ok(id) => {
                drops.clear();
                dirty(&self.world_root, who, None);
                let at = ctx.origin.map(|v| v.floor() as i32);
                match self.sc_place_grave(&id, at) {
                    Ok(p) => self.sc_private_chat(&format!(
                        "Your tombstone is at {}, {}, {}.",
                        p[0], p[1], p[2]
                    )),
                    Err(e) => self.sc_grave_error(&format!(
                        "Gravestone placement failed. Your items are cached as {id}: {e}"
                    )),
                }
            }
        }
    }
    fn sc_grave_error(&mut self, e: &str) {
        log::error!("Strangely Charming: {e}");
        self.sc_private_chat(e);
    }
    pub(super) fn sc_grave_set(&mut self, p: Pos, id: &str) -> bool {
        let Some(state) = self.pack.assets.registry.default_state(id) else {
            return false;
        };
        let [x, y, z] = p;
        if self.world.block(x, y, z) == state {
            return true;
        }
        let bp = horizon_core::position::BlockPos::new(x, y, z);
        self.with_level(|b, l, t| b.set_block(l, t, bp, state, 3));
        let _ = self.process_level_events();
        self.world.block(x, y, z) == state
    }
    fn sc_site(&self, at: Pos, reserved: &HashSet<Pos>) -> Option<(Pos, u8)> {
        let vv = candidates(at, self.world.shape.min_y, self.world.shape.max_y());
        let facts = self.res.facts();
        for mode in 0..3 {
            for &p @ [x, y, z] in &vv {
                if !self.world.loaded(x, z) {
                    continue;
                }
                let pp = [[x, y - 1, z], p, [x, y + 1, z]];
                if pp.iter().any(|q| {
                    reserved.contains(q)
                        || self.world.block_entities.contains_key(&(q[0], q[1], q[2]))
                }) {
                    continue;
                }
                let states = pp.map(|[x, y, z]| self.world.block(x, y, z));
                let air = states.map(|s| facts.is_air(s));
                let floor = facts.get(states[0]);
                let ok = match mode {
                    0 => {
                        floor.full_collision
                            && !matches!(
                                floor.name.as_ref(),
                                "minecraft:magma_block"
                                    | "minecraft:campfire"
                                    | "minecraft:soul_campfire"
                                    | "minecraft:cactus"
                            )
                            && !MODELS.contains(&floor.name.as_ref())
                            && air[1]
                            && air[2]
                    }
                    1 => air.into_iter().all(|b| b),
                    _ => states
                        .iter()
                        .enumerate()
                        .all(|(i, s)| air[i] || carve(&facts.get(*s).name)),
                };
                if ok {
                    return Some((p, mode));
                }
            }
        }
        None
    }
    fn sc_place_grave(&mut self, id: &str, at: Pos) -> Result<Pos, String> {
        let dir = self.native_world_dir().ok_or("No world directory")?;
        let (g, reserved) = access(&dir, |l| {
            (
                l.records.get(id).cloned(),
                l.records
                    .values()
                    .filter(|r| r.dimension == self.dim.id() && r.phase != "claimed")
                    .filter_map(|r| r.position)
                    .flat_map(|[x, y, z]| [[x, y, z], [x, y + 1, z]])
                    .collect::<HashSet<_>>(),
            )
        })?;
        let g = g.ok_or("Unknown death")?;
        let model = *MODELS.get(g.model).ok_or("Unknown tombstone model")?;
        if !matches!(g.phase.as_str(), "pending" | "active") {
            return Err(format!("This death is already {}.", g.phase));
        }
        if let Some(p) = g.position
            && g.dimension == self.dim.id()
        {
            if !self.world.loaded(p[0], p[2]) {
                return Err(
                    "Load the existing grave's chunk first; it may still be intact.".into(),
                );
            }
            if self.block_name(p).as_ref() == model {
                return Err(format!("An intact grave already exists at {p:?}"));
            }
        } else if g.position.is_some() {
            return Err("Check the existing grave in its dimension first.".into());
        }
        let (p @ [x, y, z], mode) = self
            .sc_site(at, &reserved)
            .ok_or("No safe site within ten blocks")?;
        change(&dir, |l| {
            let g = l.records.get_mut(id).ok_or("Unknown death")?;
            g.position = Some(p);
            g.dimension = self.dim.id().into();
            g.phase = "active".into();
            Ok(())
        })?;
        // Only natural terrain in the explicit allowlist may be carved. No container,
        // portal, bed, fluid, protected marker, or unfamiliar mod block is replaceable.
        let edits = [
            ([x, y - 1, z], "minecraft:dirt", mode != 0),
            ([x, y + 1, z], AIR, mode == 2),
            (p, model, true),
        ];
        let mut undo: Vec<(Pos, String)> = Vec::new();
        for (q, name, needed) in edits {
            if !needed {
                continue;
            }
            let previous = self.block_name(q).to_string();
            if !self.sc_grave_set(q, name) {
                for (q, old) in undo.into_iter().rev() {
                    self.sc_grave_set(q, &old);
                }
                change(&dir, |l| {
                    let g = l.records.get_mut(id).ok_or("Unknown death")?;
                    g.position = None;
                    g.phase = "pending".into();
                    Ok(())
                })?;
                return Err("World rejected a placement".into());
            }
            undo.push((q, previous));
        }
        Ok(p)
    }
    fn sc_marker(&self, p: Pos) -> Result<Option<Grave>, String> {
        let name = self.block_name(p);
        if !MODELS.contains(&name.as_ref()) || self.remote.is_some() {
            return Ok(None);
        }
        let Some(dir) = self.native_world_dir() else {
            return Ok(None);
        };
        access(&dir, |l| {
            l.records
                .values()
                .find(|g| {
                    g.position == Some(p)
                        && g.dimension == self.dim.id()
                        && g.phase != "claimed"
                        && MODELS.get(g.model) == Some(&name.as_ref())
                })
                .cloned()
        })
    }
    pub(super) fn sc_grave_rule(&self, p: Pos) -> Result<MiningRule, String> {
        Ok(self.sc_marker(p)?.map_or(MiningRule::Decoration, |g| {
            if g.phase != "active" {
                MiningRule::Unavailable
            } else {
                MiningRule::Grave(
                    g.break_after_seconds
                        .unwrap_or(7200)
                        .saturating_sub(now().saturating_sub(g.death_utc)),
                )
            }
        }))
    }
    pub(super) fn sc_grave_break(&mut self, p: Pos) -> bool {
        match self.sc_grave_rule(p) {
            Ok(rule) => {
                if let Some(message) = rule.warning(Duration::ZERO, self.sc_has_hammer()) {
                    self.sc_notice(message);
                    true
                } else {
                    false
                }
            }
            Err(e) => {
                self.sc_grave_error(&e);
                true
            }
        }
    }
    fn sc_test_break_grave(&mut self, at: Pos) -> Result<String, String> {
        if !horizon_mods::evanders_config::enabled("gravestones") {
            return Err("Gravestones are disabled.".into());
        }
        let dir = self.native_world_dir().ok_or("No world directory")?;
        let name = self.acting_player_name().to_owned();
        let t = now();
        let id = format!("test-{}", death_id(&name, t)?);
        // Twenty full stacks in total, with four half-stacks to exercise merging.
        let slots = (0..22)
            .map(|i| {
                Slot::save(
                    i,
                    &ItemStack::new("minecraft:sugar_cane", if i < 18 { 64 } else { 32 }),
                )
            })
            .collect();
        let grave = Grave {
            id: id.clone(),
            owner: format!("{:032x}", self.native_acting_id()),
            name,
            death_utc: t,
            break_after_seconds: Some(60),
            sequence: 0,
            model: self.world.random.next_int_bound(3).unwrap_or(0) as usize,
            dimension: self.dim.id().into(),
            death: at,
            position: None,
            phase: "pending".into(),
            slots,
            displaced: vec![],
        };
        change(&dir, |l| {
            if l.records.contains_key(&id) {
                return Err("Grave ID collision".into());
            }
            l.remember(grave);
            Ok(())
        })?;
        let pos = self.sc_place_grave(&id, at)?;
        Ok(format!(
            "Test grave at {pos:?}. Break it with a Runebreaker after one minute to release 1,280 sugar cane."
        ))
    }

    // Called only on the completed break, after other native mods can veto it.
    pub(super) fn sc_spill_broken_grave(&mut self, p: Pos) -> bool {
        let g = match self.sc_marker(p) {
            Ok(Some(g)) => g,
            Ok(None) => return false,
            Err(e) => {
                self.sc_grave_error(&e);
                return true;
            }
        };
        let release = (|| -> Result<(), String> {
            let dir = self.native_world_dir().ok_or("No world directory")?;
            let mut piles: Vec<ItemStack> = Vec::new();
            for slot in &g.slots {
                let mut stack = slot.load()?;
                // ItemEntity.mergeWithNeighbours only joins matching components when
                // the whole pair fits. Its normal transfer target is capped at 64.
                let limit = self.stack_max_size(&stack).max(1);
                for pile in &mut piles {
                    if !pile.same_item_same_components(&stack)
                        || u64::from(pile.count) + u64::from(stack.count) > u64::from(limit)
                    {
                        continue;
                    }
                    if pile.count < stack.count {
                        std::mem::swap(pile, &mut stack);
                    }
                    let moved = stack.count.min(limit.min(64).saturating_sub(pile.count));
                    pile.count += moved;
                    stack.count -= moved;
                    if stack.count == 0 {
                        break;
                    }
                }
                if stack.count > 0 {
                    piles.push(stack);
                }
            }
            change(&dir, |l| {
                let record = l.records.get_mut(&g.id).ok_or("Unknown death")?;
                if record.phase != "active" || record.position != Some(p) {
                    return Err(
                        "This grave has already been opened or needs operator inspection.".into(),
                    );
                }
                record.phase = "spilling".into();
                Ok(())
            })?;
            if !self.sc_grave_set(p, AIR) {
                change(&dir, |l| {
                    l.records.get_mut(&g.id).unwrap().phase = "active".into();
                    Ok(())
                })?;
                return Err("World rejected removal of the grave.".into());
            }
            let center = p.map(|n| f64::from(n) + 0.5);
            self.spawn_loaded_entities();
            for stack in piles {
                self.spawn_item(stack, center, [0.; 3], 20);
                if self.just_spawned_item().is_none() {
                    return Err(format!(
                        "A grave item could not enter the world. Inspect death {} before reopening.",
                        g.id
                    ));
                }
            }
            // Persist only the affected entity chunk before closing the recovery record.
            let chunk = (p[0] >> 4, p[2] >> 4);
            let data = self.entity_chunks(&HashSet::from([chunk]), false);
            self.save.save_entities(data).map_err(|e| e.to_string())?;
            change(&dir, |l| {
                let record = l.records.get_mut(&g.id).unwrap();
                record.phase = "claimed".into();
                record.position = None;
                l.prune();
                Ok(())
            })?;
            if let Ok(owner) = u128::from_str_radix(&g.owner, 16) {
                // Use the owner's connection even when someone else broke the grave.
                veil::stop_owner(&dir, owner);
            }
            Ok(())
        })();
        if let Err(e) = release {
            self.sc_grave_error(&e);
        }
        true
    }

    pub(super) fn sc_grave_use(&mut self, p: Pos) -> bool {
        let g = match self.sc_marker(p) {
            Ok(Some(g)) => g,
            Ok(None) => return false,
            Err(e) => {
                self.sc_grave_error(&e);
                return true;
            }
        };
        let who = self.native_acting_id();
        if g.owner != format!("{who:032x}") {
            self.sc_notice(format!("only {} may recover this grave", g.name));
            return true;
        }
        let restore = (|| -> Result<(), String> {
            let dir = self.native_world_dir().ok_or("No world directory")?;
            if g.phase != "active" {
                return Err("This claim needs an operator to inspect the recovery ledger.".into());
            }
            let mut restored = Vec::new();
            let mut seen = HashSet::new();
            for a in &g.slots {
                if !seen.insert(a.slot) {
                    return Err("Duplicate slot in grave ledger".into());
                }
                restored.push((a.slot, a.load()?));
            }
            let displaced = self.sc_slots();
            let escrow = displaced.iter().map(|(i, s)| Slot::save(*i, s)).collect();
            change(&dir, |l| {
                let g = l.records.get_mut(&g.id).ok_or("Unknown death")?;
                if g.phase != "active" {
                    return Err("Already claimed".into());
                }
                g.phase = "claiming".into();
                g.displaced = escrow;
                Ok(())
            })?;
            veil::stop(who, self.lan_acting);
            let was_visible = GUARDS
                .get()
                .and_then(|guards| guards.lock().ok())
                .and_then(|mut guards| {
                    guards.get_mut(&(dir.clone(), who)).map(|s| {
                        s.last = None;
                        std::mem::take(&mut s.wayfinder_visible)
                    })
                })
                .unwrap_or(false);
            if was_visible {
                self.sc_send_grave_view(
                    who,
                    View::Bearing {
                        dimension: self.dim.id().into(),
                        grave: None,
                    },
                );
            }
            for i in 0..=40 {
                *self.sc_slot_mut(i) = None;
            }
            let center = [p[0] as f64 + 0.5, p[1] as f64 + 0.5, p[2] as f64 + 0.5];
            for (i, s) in restored {
                if i <= 40 {
                    *self.sc_slot_mut(i) = Some(s);
                } else {
                    self.spawn_item(s, center, [0.; 3], 20);
                }
            }
            self.apply_equipment();
            for (_, s) in displaced {
                self.spawn_item(s, center, [0.; 3], 20);
            }
            self.sc_grave_set(p, AIR);
            if let Some(level) = self.native_level(self.dim.id()) {
                use horizon_client::mod_api::{ParticleOptions, ParticleType};
                self.native_level_play_sound(
                    level,
                    "minecraft:block.deepslate.break",
                    center,
                    4,
                    1.,
                    1.,
                );
                self.native_level_send_particles(
                    level,
                    &ParticleOptions::simple(ParticleType::CampfireCosySmoke),
                    None,
                    [center[0], center[1] + 0.35, center[2]],
                    [0.2, 0.1, 0.2],
                    0.012,
                    5,
                );
            }
            dirty(&dir, who, Some(g.id.clone()));
            Ok(())
        })();
        if let Err(e) = restore {
            self.sc_grave_error(&e)
        }
        true
    }
    pub(super) fn sc_grave_tick(&mut self) {
        if self.remote.is_some() || !horizon_mods::evanders_config::enabled("gravestones") {
            return;
        }
        let Some(p) = self
            .player
            .as_ref()
            .filter(|p| !p.health.dead_or_dying())
            .map(|p| p.position)
        else {
            return;
        };
        let Some(dir) = self.native_world_dir() else {
            return;
        };
        let who = self.native_acting_id();
        // Expire on the server's simulation clock, including ticks between scans.
        veil::active(&dir, who, self.lan_acting, self.world.game_time);
        let mut all = GUARDS.get_or_init(Default::default).lock().unwrap();
        let s = all.entry((dir.clone(), who)).or_default();
        if s.last
            .is_some_and(|t| t.elapsed() < Duration::from_millis(400))
        {
            return;
        }
        s.last = Some(Instant::now());
        let owner = format!("{who:032x}");
        let Ok((wayfinder, near)) = access(&dir, |l| {
            let marker = l
                .latest_active(&owner)
                .filter(|g| g.dimension == self.dim.id())
                .and_then(|g| {
                    let pos = g.position?;
                    // A short render distance can unload a grave still within 100 blocks.
                    (!self.world.loaded(pos[0], pos[2])
                        || MODELS.get(g.model) == Some(&self.block_name(pos).as_ref()))
                    .then_some(pos)
                });
            let line = marker
                .filter(|_| horizon_mods::evanders_config::enabled("grave_wayfinder"))
                .filter(|pos| {
                    let [x, y, z] = grave_center(*pos);
                    (x - p.x).hypot(z - p.z).hypot(y - p.y) <= 100.
                });
            let near = l
                .records
                .values()
                .filter(|g| {
                    g.owner == owner
                        && g.phase == "active"
                        && g.dimension == self.dim.id()
                        && !s.seen.contains(&g.id)
                })
                .find_map(|g| {
                    let [x, y, z] = g.position?;
                    let d = (p.x - x as f64 - 0.5).powi(2)
                        + (p.y - y as f64).powi(2)
                        + (p.z - z as f64 - 0.5).powi(2);
                    (d <= 1024. && self.block_name([x, y, z]).as_ref() == MODELS[g.model])
                        .then(|| g.id.clone())
                });
            (line, near)
        }) else {
            return;
        };
        let clear = s.wayfinder_visible && wayfinder.is_none();
        s.wayfinder_visible = wayfinder.is_some();
        if let Some(id) = &near {
            s.seen.insert(id.clone());
        }
        drop(all);
        if let Some(pos) = wayfinder {
            self.sc_send_grave_view(
                who,
                View::Bearing {
                    dimension: self.dim.id().into(),
                    grave: Some(pos),
                },
            );
        } else if clear {
            self.sc_send_grave_view(
                who,
                View::Bearing {
                    dimension: self.dim.id().into(),
                    grave: None,
                },
            );
        }
        if near.is_some() {
            veil::start(&dir, who, self.lan_acting, self.world.game_time);
            self.native_play_sound_to(
                who,
                "minecraft:entity.happy_ghast.ambient",
                "neutral",
                None,
                0.8,
                1.,
            );
            self.sc_private_chat(
                "your tombstone is hiding you from monsters, hurry and recover your things!",
            );
        }
    }
    pub(super) fn sc_shielded(&self) -> bool {
        let who = self.lan_acting_uuid.or_else(|| {
            self.level
                .player_uuid
                .map(horizon_client::lan::playerdata::ints_uuid)
        });
        who.is_some_and(|who| {
            veil::active(&self.world_root, who, self.lan_acting, self.world.game_time)
        })
    }
    pub(super) fn sc_grave_command(&mut self, cmd: &str, at: Option<Pos>) -> Option<String> {
        let mut w = cmd.trim_start_matches('/').split_whitespace();
        let label = w.next()?;
        if label != "scgraves" {
            return None;
        }
        let result = (|| -> Result<String, String> {
            let dir = self.native_world_dir().ok_or("No world open")?;
            match (w.next(), w.next(), w.next()) {
                (Some(mode @ ("simulate-death" | "simulate-loop")), target, None) =>
                    self.sc_simulate_target(target.unwrap_or("@s"), at.is_some(), mode == "simulate-loop"),
                (Some("test-break"), None, None) => {
                    self.sc_test_break_grave(at.ok_or("Run this command as a player at the desired grave site.")?)
                }
                (Some("list"), None, None) => access(&dir, |l| {
                    let mut ids = l.recent.clone();
                    ids.extend(
                        l.records
                            .keys()
                            .filter(|id| !l.recent.contains(id))
                            .cloned(),
                    );
                    if ids.is_empty() {
                        "No cached deaths.".into()
                    } else {
                        ids.into_iter()
                            .filter_map(|i| {
                                l.records.get(&i).map(|g| {
                                    format!(
                                        "{}: {} ({}, {:?})",
                                        i, g.phase, g.dimension, g.position
                                    )
                                })
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                }),
                // Explicit operator-only recovery when a crash or world rollback left the claim
                // ambiguous. Never reopen automatically, and preserve the displaced-item audit.
                (Some("reopen"), Some(id), Some("confirmed")) if w.next().is_none() => {
                    change(&dir, |l| {
                        let g = l.records.get_mut(id).ok_or("Unknown death")?;
                        if !matches!(g.phase.as_str(), "claimed" | "claiming" | "spilling") {
                            return Err("This death is still recoverable; use spawn.".into());
                        }
                        g.phase = "pending".into();
                        g.position = None;
                        Ok(())
                    })?;
                    log::warn!("Operator explicitly reopened death {id}");
                    Ok(format!(
                        "Reopened {id}. Use /scgraves spawn {id} to place it."
                    ))
                }
                (Some("spawn"), Some(id), None) => {
                    let g =
                        access(&dir, |l| l.records.get(id).cloned())?.ok_or("Unknown death ID")?;
                    if at.is_none() && g.dimension != self.dim.id() {
                        return Err("Use an operator in the grave's dimension.".into());
                    }
                    let p = self.sc_place_grave(id, at.unwrap_or(g.death))?;
                    Ok(format!("Placed {} at {p:?}", g.id))
                }
                _ => Ok("Usage: /scgraves list | spawn <death-id> | reopen <death-id> confirmed | simulate-death [player] | simulate-loop [player] | test-break".into()),
            }
        })();
        Some(result.unwrap_or_else(|e| format!("Graves: {e}")))
    }
}

pub(super) const TEST_MOB: &str = "strangely_charming:grave_test_mob";
const TEST_MOBS: [&str; 4] = [
    "minecraft:zombie",
    "minecraft:skeleton",
    "minecraft:spider",
    "minecraft:creeper",
];

fn grave_center([x, y, z]: Pos) -> [f64; 3] {
    [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5]
}

pub(super) fn grave_bearing(from: [f64; 3], yaw: f32, grave: Pos) -> Option<(String, [u8; 3])> {
    let (distance, rgb) = waypoint_bearing(from, yaw, grave);
    (distance <= 100.).then(|| (format!("Grave: {distance:.0} blocks"), rgb))
}
pub(super) fn waypoint_bearing(from: [f64; 3], yaw: f32, target: Pos) -> (f64, [u8; 3]) {
    let [x, y, z] = grave_center(target);
    let (dx, dy, dz) = (x - from[0], y - from[1], z - from[2]);
    let horizontal = dx.hypot(dz);
    let distance = horizontal.hypot(dy);
    let angle = if horizontal < 0.001 {
        0.
    } else {
        ((-dx).atan2(dz).to_degrees() - f64::from(yaw) + 180.).rem_euclid(360.) - 180.
    };
    let error = angle.abs();
    let rgb = [
        (255. * (error / 90.).min(1.)).round() as u8,
        (255. * ((180. - error) / 90.).min(1.)).round() as u8,
        0,
    ];
    (distance, rgb)
}

struct LoopSites {
    landing: [f64; 3],
    mobs: [[f64; 3]; 4],
}

impl Game<'_> {
    fn sc_simulate_target(
        &mut self,
        name: &str,
        player_source: bool,
        loop_test: bool,
    ) -> Result<String, String> {
        if !horizon_mods::evanders_config::enabled("gravestones") {
            return Err("Gravestones are disabled.".into());
        }
        let target = if name == "@s" {
            if !player_source {
                return Err("Supply an online player name from the console.".into());
            }
            super::commands::Target::Player
        } else {
            let player = self
                .native_server_players()
                .into_iter()
                .find(|p| p.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("No online player named {name}."))?;
            self.target_by_uuid(player.uuid)
                .ok_or("Player is no longer online.")?
        };
        let dimension = self.target_level(target);
        self.for_player(target, |g| {
            if g.dim != dimension {
                return Err("Could not enter the target player's dimension.".into());
            }
            g.sc_simulate_death(loop_test)
        })
        .ok_or("Could not access the target player's inventory.")?
    }

    fn sc_test_footing(&self, at: [f64; 3], half_width: f64) -> bool {
        let [x, y, z] = at;
        let y = y as i32;
        let [min_x, min_z, max_x, max_z] = self.border.bounds();
        if y <= self.world.shape.min_y
            || y + 1 >= self.world.shape.max_y()
            || x - half_width < min_x
            || x + half_width >= max_x
            || z - half_width < min_z
            || z + half_width >= max_z
        {
            return false;
        }
        let facts = self.res.facts();
        for bx in (x - half_width).floor() as i32..=(x + half_width).floor() as i32 {
            for bz in (z - half_width).floor() as i32..=(z + half_width).floor() as i32 {
                if !self.world.loaded(bx, bz) {
                    return false;
                }
                let floor = facts.get(self.world.block(bx, y - 1, bz));
                if !floor.full_collision
                    || is_marker(&floor.name)
                    || matches!(
                        floor.name.as_ref(),
                        "minecraft:magma_block"
                            | "minecraft:campfire"
                            | "minecraft:soul_campfire"
                            | "minecraft:cactus"
                    )
                    || !facts.is_air(self.world.block(bx, y, bz))
                    || !facts.is_air(self.world.block(bx, y + 1, bz))
                {
                    return false;
                }
            }
        }
        true
    }

    fn sc_loop_sites(&self, grave: Pos, other_graves: &[Pos]) -> Result<LoopSites, String> {
        if self.ui.difficulty == 0 {
            return Err("Set the difficulty above Peaceful to simulate a death loop.".into());
        }
        let [x, y, z] = grave_center(grave);
        // Search a 64-block sphere, preferring level ground and small height differences.
        let landing = (0..=32)
            .flat_map(|h| [h, -h])
            .find_map(|height| {
                let dy = f64::from(height);
                let radius = (64.0_f64.powi(2) - dy * dy).sqrt();
                (0..64).find_map(|i| {
                    let (sin, cos) = (std::f64::consts::TAU * f64::from(i) / 64.0).sin_cos();
                    let at = [x + radius * cos, y + dy, z + radius * sin];
                    let outside_shields = other_graves.iter().all(|&p| {
                        let [gx, gy, gz] = grave_center(p);
                        (at[0] - gx).powi(2) + (at[1] - gy).powi(2) + (at[2] - gz).powi(2)
                            > 32.0_f64.powi(2)
                    });
                    (outside_shields && self.sc_test_footing(at, 0.3)).then_some(at)
                })
            })
            .ok_or("No safe landing 64 blocks from this grave. Try a more open area.")?;
        let mut mobs = [[0.0; 3]; 4];
        for (i, site) in mobs.iter_mut().enumerate() {
            *site = (0..=5)
                .flat_map(|h| [h, -h])
                .find_map(|dy| {
                    (4..=9).find_map(|r| {
                        (0..5).find_map(|j| {
                            let angle =
                                (i as f64 * 90.0 + [0.0, 15.0, -15.0, 30.0, -30.0][j]).to_radians();
                            let (sin, cos) = angle.sin_cos();
                            let at = [
                                x + (f64::from(r) * cos).round(),
                                y + f64::from(dy),
                                z + (f64::from(r) * sin).round(),
                            ];
                            self.sc_test_footing(at, if i == 2 { 0.71 } else { 0.31 })
                                .then_some(at)
                        })
                    })
                })
                .ok_or("Not enough safe space for the four mobs around this grave.")?;
        }
        Ok(LoopSites { landing, mobs })
    }

    fn sc_simulate_death(&mut self, loop_test: bool) -> Result<String, String> {
        let player = self
            .player
            .as_ref()
            .filter(|p| !p.health.dead_or_dying())
            .ok_or("Target must be alive.")?;
        let origin = [player.position.x, player.position.y, player.position.z];
        let entity = self
            .lan_acting_entity
            .ok_or("Target must be a connected player.")?;
        if self.ui.rules.bool("keep_inventory") {
            return Err("keepInventory is on. Disable it to test normal death drops.".into());
        }
        let who = self.native_acting_id();
        let dir = self.native_world_dir().ok_or("No world directory")?;
        let owner = format!("{who:032x}");
        let (last, reserved, other) = access(&dir, |l| {
            let mut reserved = HashSet::new();
            let mut other = Vec::new();
            for g in l
                .records
                .values()
                .filter(|g| g.dimension == self.dim.id() && g.phase != "claimed")
            {
                if let Some(p @ [x, y, z]) = g.position {
                    reserved.extend([p, [x, y + 1, z]]);
                    if g.owner == owner && g.phase == "active" {
                        other.push(p);
                    }
                }
            }
            (l.recent.first().cloned(), reserved, other)
        })?;
        let (site, _) = self
            .sc_site(origin.map(|c| c.floor() as i32), &reserved)
            .ok_or("No safe grave site within ten blocks.")?;
        let plan = if loop_test {
            Some(self.sc_loop_sites(site, &other)?)
        } else {
            None
        };
        // The real death-drop hook handles components, slots, other loot hooks,
        // checkpointing and fallback drops. Never call /kill or touch health or XP.
        self.drop_inventory(origin, (who, entity as i32));
        let grave = access(&dir, |l| {
            l.recent
                .first()
                .filter(|id| Some(*id) != last.as_ref())
                .and_then(|id| l.records.get(id))
                .filter(|g| g.owner == owner)
                .map(|g| (g.id.clone(), g.position, g.phase.clone()))
        })?
        .ok_or(
            "The grave was not recorded. Check the target player's death drops and the server log.",
        )?;
        let (id, pos, phase) = grave;
        let pos = pos
            .filter(|_| phase == "active")
            .ok_or_else(|| format!("Death {id} is cached but its marker could not be placed."))?;
        let name = self.acting_player_name().to_owned();
        if let Some(plan) = plan {
            if pos != site {
                return Err(format!(
                    "Grave {id} was placed at {pos:?}. Terrain changed during the simulation."
                ));
            }
            let dim = self.dim.id();
            if !self.native_teleport_player(who, dim, plan.landing, self.yaw, self.pitch) {
                return Err(format!(
                    "Grave {id} is at {pos:?}, but teleportation failed."
                ));
            }
            let mut spawned = Vec::new();
            for (kind, at) in TEST_MOBS.into_iter().zip(plan.mobs) {
                let Some(mob_id) = self.native_spawn_mob(dim, kind, at, &Default::default()) else {
                    for id in spawned {
                        self.native_remove_mob(dim, id);
                    }
                    return Err(format!(
                        "Grave {id} is recoverable at {pos:?}. Could not spawn {kind}."
                    ));
                };
                spawned.push(mob_id);
                let mob = self
                    .mobs
                    .iter_mut()
                    .find(|m| m.id == mob_id)
                    .expect("spawned native mob");
                std::sync::Arc::make_mut(&mut mob.kind).flags.burns_in_day = false;
                mob.persistent = false;
                let raw = self
                    .persist
                    .raw
                    .entry(mob_id)
                    .or_insert_with(|| Tag::Compound(Default::default()));
                if let Tag::Compound(data) = raw {
                    data.insert(TEST_MOB.into(), Tag::Byte(1));
                }
            }
            Ok(format!(
                "Death loop for {name}: grave at {pos:?}, player 64 blocks away, four daylight-safe mobs."
            ))
        } else {
            Ok(format!("Simulated death for {name}. Grave at {pos:?}."))
        }
    }
}
