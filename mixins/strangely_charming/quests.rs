use super::{
    Game,
    sc_items::{custom, set_custom},
};
use horizon_client::{items::ItemStack, mobs::Species};
use horizon_world::nbt::Tag;
use serde::{Deserialize, Serialize};

const ISSUER: &str = "strangely_charming:issuer";
const CONTRACT: &str = "strangely_charming:contract";
const OWNER: &str = "strangely_charming:contract_owner";
const CHANNEL: &str = "strangely_charming:quest/";
fn string(v: impl Into<String>) -> Tag {
    Tag::String(v.into().as_str().into())
}
fn text_value(t: Option<&Tag>) -> Option<String> {
    if let Some(Tag::String(s)) = t {
        Some(s.to_string_lossy())
    } else {
        None
    }
}
#[derive(Copy, Clone, PartialEq, Serialize, Deserialize)]
pub(super) enum Kind {
    Wheat,
    Bread,
    Coal,
    Zombies,
    Books,
}
impl Kind {
    pub(super) fn item(id: &str) -> Option<Self> {
        Some(match id.strip_prefix("strangely_charming:scroll_")? {
            "wheat" => Self::Wheat,
            "bread" => Self::Bread,
            "coal" => Self::Coal,
            "zombies" => Self::Zombies,
            "books" => Self::Books,
            _ => return None,
        })
    }
    pub(super) fn objective(self) -> &'static str {
        match self {
            Self::Wheat => "obtain 12 wheat",
            Self::Bread => "bake 8 loaves of bread",
            Self::Coal => "mine 12 blocks of coal ore or deepslate coal ore",
            Self::Zombies => "defeat 5 zombies yourself or with your tamed wolf",
            Self::Books => "obtain 16 books",
        }
    }
    pub(super) fn target(self) -> u32 {
        match self {
            Self::Wheat | Self::Coal => 12,
            Self::Bread => 8,
            Self::Zombies => 5,
            Self::Books => 16,
        }
    }
    pub(super) fn reward(self) -> u32 {
        match self {
            Self::Wheat => 3,
            Self::Bread => 5,
            Self::Coal => 8,
            Self::Zombies => 10,
            Self::Books => 12,
        }
    }
    pub(super) fn hero_rank(self) -> usize {
        match self {
            Self::Wheat => 1,
            Self::Bread => 2,
            Self::Coal => 3,
            Self::Zombies | Self::Books => 4,
        }
    }
    pub(super) fn hero_name(self) -> &'static str {
        ["I", "II", "III", "IV"][self.hero_rank() - 1]
    }
    fn delivery(self) -> Option<&'static str> {
        match self {
            Self::Wheat => Some("minecraft:wheat"),
            Self::Books => Some("minecraft:book"),
            _ => None,
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Contract {
    serial: u64,
    kind: Kind,
    progress: u32,
    #[serde(default)]
    notified: bool,
}
#[derive(Serialize, Deserialize, Default)]
struct Ledger {
    serial: u64,
    active: Vec<Contract>,
    preview: Option<(u64, usize, Kind, Option<String>)>,
}
fn button(label: impl Into<String>, action: &str) -> Tag {
    Tag::compound([
        ("label", string(label)),
        (
            "action",
            Tag::compound([
                ("type", string("minecraft:custom")),
                ("id", string(format!("{CHANNEL}{action}"))),
            ]),
        ),
    ])
}
impl Game<'_> {
    fn sc_record(&mut self) -> String {
        format!(
            "strangely_charming:contracts/{:032x}",
            self.native_acting_id()
        )
    }
    fn sc_load(&mut self) -> Ledger {
        let id = self.sc_record();
        text_value(self.score.storage.get(&id).get("ledger"))
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    fn sc_save(&mut self, l: &Ledger) {
        let id = self.sc_record();
        self.score.storage.set(
            &id,
            Tag::compound([
                (
                    "watch",
                    Tag::Byte(i8::from(l.active.iter().any(|c| !c.notified))),
                ),
                (
                    "ledger",
                    string(serde_json::to_string(l).expect("quest ledger")),
                ),
            ]),
        );
    }
    fn sc_dialog(&mut self, title: &str, body: String, mut actions: Vec<Tag>) {
        actions.push(Tag::compound([("label", string("Close"))]));
        self.show_click_dialog(&Tag::compound([
            ("type", string("minecraft:multi_action")),
            ("title", string(title)),
            ("columns", Tag::Int(1)),
            (
                "body",
                Tag::compound([
                    ("type", string("minecraft:plain_message")),
                    ("contents", string(body)),
                    ("width", Tag::Int(330)),
                ]),
            ),
            ("actions", Tag::List(actions)),
            ("pause", Tag::Byte(0)),
        ]));
    }
    fn sc_keeper_id(&mut self, id: u32) -> String {
        format!(
            "{:032x}",
            horizon_client::lan::playerdata::ints_uuid(self.mob_uuid(id))
        )
    }
    pub(super) fn sc_stamp_scrolls(&mut self, i: usize) {
        let id = self.sc_keeper_id(self.mobs[i].id);
        let m = &mut self.mobs[i];
        if let Some(t) = &mut m.extra.trade {
            for offer in &mut t.offers {
                if let Some(kind) = Kind::item(&offer.result.item) {
                    // Saved offers from 0.2.39 retain their original zero XP until reopened.
                    if offer.xp == 0 {
                        offer.xp = match kind {
                            Kind::Wheat => 2, Kind::Bread => 5, Kind::Coal => 10,
                            Kind::Zombies => 15, Kind::Books => 20,
                        };
                    }
                    set_custom(&mut offer.result, ISSUER, string(&id));
                }
            }
        }
    }
    pub(super) fn sc_scroll(&mut self, stack: &ItemStack, kind: Kind) {
        let mut ledger = self.sc_load();
        let preview = if let Some(Tag::Long(serial)) = custom(stack, CONTRACT) {
            let owner = format!("{:032x}", self.native_acting_id());
            if text_value(custom(stack, OWNER)).as_deref() != Some(&owner) {
                self.sc_notice("This scroll belongs to another player.");
                return;
            }
            let Some(contract) = ledger.active.iter().find(|c| c.serial == *serial as u64 && c.kind == kind) else {
                self.sc_notice("This contract is no longer active.");
                return;
            };
            super::sc_journal::QuestPreview {
                serial: contract.serial,
                kind,
                progress: Some(self.sc_progress(contract)),
            }
        } else {
            ledger.serial = ledger.serial.saturating_add(1);
            let serial = ledger.serial;
            // The seller stamp identifies the unsigned offer while the confirmation is open.
            ledger.preview = Some((serial, self.held_slot(), kind, text_value(custom(stack, ISSUER))));
            self.sc_save(&ledger);
            super::sc_journal::QuestPreview {
                serial,
                kind,
                progress: None,
            }
        };
        let data = serde_json::to_vec(&preview).expect("serializable quest preview");
        if self.lan_acting_uuid.is_some() {
            use horizon_protocol::packets::{configuration::CustomPayload, play::Clientbound};
            let who = self.native_acting_id();
            self.native_send(who, Clientbound::CustomPayload(CustomPayload {
                channel: super::sc_journal::QUEST_VIEW.into(), data,
            }));
        } else {
            self.sc_receive_quest_preview(&data);
        }
    }
    fn sc_progress(&mut self, c: &Contract) -> u32 {
        c.kind
            .delivery()
            .map_or(c.progress, |item| self.native_count_items(item))
            .min(c.kind.target())
    }
    pub(super) fn sc_event(&mut self, kind: Kind, count: u32) {
        if !horizon_mods::evanders_config::enabled("scrollkeepers") || self.remote.is_some() {
            return;
        }
        let mut l = self.sc_load();
        let mut changed = false;
        for c in &mut l.active {
            if c.kind == kind && c.progress < kind.target() {
                c.progress = c.progress.saturating_add(count).min(kind.target());
                changed = true;
            }
        }
        if changed {
            self.sc_mark_ready(&mut l);
            self.sc_save(&l);
        }
    }
    fn sc_mark_ready(&mut self, ledger: &mut Ledger) -> bool {
        let mut changed = false;
        for c in &mut ledger.active {
            if !c.notified && self.sc_progress(c) == c.kind.target() {
                c.notified = true;
                changed = true;
                let opening = if c.kind.delivery().is_some() {
                    "You found all the items!"
                } else {
                    "Quest complete!"
                };
                let own_scroll = self
                    .inventory
                    .iter()
                    .flatten()
                    .chain(self.offhand.iter())
                    .any(|s| custom(s, CONTRACT) == Some(&Tag::Long(c.serial as i64)));
                let instruction = if own_scroll {
                    "Sneak right click your scroll on a Scrollkeeper to turn it in."
                } else {
                    "Sneak right click any Scrollkeeper and choose Claim in the journal."
                };
                self.sc_alert(format!("{opening} {instruction}"), true);
            }
        }
        changed
    }
    pub(super) fn sc_check_ready(&mut self) {
        if !horizon_mods::evanders_config::enabled("scrollkeepers") || self.player.is_none() {
            return;
        }
        let id = self.sc_record();
        let record = self.score.storage.get(&id);
        if record.get("ledger").is_none() || matches!(record.get("watch"), Some(Tag::Byte(0))) {
            return;
        }
        let mut ledger = self.sc_load();
        if self.sc_mark_ready(&mut ledger) {
            self.sc_save(&ledger);
        }
    }
    fn sc_remove_scroll(&mut self, serial: u64) {
        let owner = format!("{:032x}", self.native_acting_id());
        for slot in self
            .inventory
            .iter_mut()
            .chain(std::iter::once(&mut self.offhand))
        {
            if slot.as_ref().is_some_and(|s| {
                custom(s, CONTRACT) == Some(&Tag::Long(serial as i64))
                    && text_value(custom(s, OWNER)).as_ref() == Some(&owner)
            }) {
                *slot = None;
            }
        }
    }
    pub(super) fn sc_quest_desk(&mut self, index: usize) {
        let id = self.sc_keeper_id(self.mobs[index].id);
        let owner = format!("{:032x}", self.native_acting_id());
        if let Some(s) = self.held()
            && let Some(Tag::Long(serial)) = custom(s, CONTRACT)
        {
            if text_value(custom(s, OWNER)) == Some(owner) {
                self.sc_quest_action(&format!("claim/{serial}/{id}"));
            } else {
                self.sc_notice("This scroll belongs to another player.");
            }
            return;
        }
        self.sc_journal(Some(&id));
    }
    pub(super) fn sc_journal(&mut self, keeper: Option<&str>) {
        let ledger = self.sc_load();
        let mut body = format!("Active quests: {} / 2", ledger.active.len());
        let mut actions = Vec::new();
        for c in &ledger.active {
            let n = self.sc_progress(c);
            let target = c.kind.target();
            let fill = (8 * n / target) as usize;
            body.push_str(&format!(
                "\n\n#{}  {}\n[{}{}]  {n}/{target}  •  Any Scrollkeeper\nReward: {} emeralds and {} XP",
                c.serial,
                c.kind.objective(),
                "|".repeat(fill),
                ".".repeat(8 - fill),
                c.kind.reward(),
                c.kind.reward() * 3
            ));
            if n == target
                && let Some(id) = keeper
            {
                actions.push(button(
                    format!("Claim #{}", c.serial),
                    &format!("claim/{}/{id}", c.serial),
                ));
            }
            actions.push(button(
                format!("Abandon #{}", c.serial),
                &format!("abandon/{}", c.serial),
            ));
        }
        if ledger.active.is_empty() {
            body.push_str("\n\nRead a scroll to accept a local quest.");
        }
        self.sc_dialog("Scrollkeeper journal", body, actions);
    }
    pub(super) fn sc_quest_action(&mut self, action: &str) {
        if !horizon_mods::evanders_config::enabled("scrollkeepers") || self.player.is_none() {
            return;
        }
        if action == "badges" {
            self.sc_send_badges();
            return;
        }
        if action == "journal" {
            self.sc_journal(None);
            return;
        }
        let mut parts = action.split('/');
        let (Some(verb), Some(serial)) = (
            parts.next(),
            parts.next().and_then(|s| s.parse::<u64>().ok()),
        ) else {
            return;
        };
        let mut l = self.sc_load();
        if matches!(verb, "accept" | "decline") {
            let Some((token, slot, kind, issuer)) = &l.preview else {
                return;
            };
            if *token != serial {
                return;
            }
            let Some(stack) = self.stack_in(*slot) else {
                return;
            };
            if Kind::item(&stack.item) != Some(*kind)
                || custom(stack, CONTRACT).is_some()
                || text_value(custom(stack, ISSUER)) != *issuer
            {
                return;
            }
            if verb == "accept" {
                if l.active.len() >= 2 {
                    self.sc_notice("Both quest slots are occupied.");
                    return;
                }
                l.active.push(Contract {
                    serial,
                    kind: *kind,
                    progress: 0,
                    notified: false,
                });
                let owner = format!("{:032x}", self.native_acting_id());
                let scroll = self.stack_in_mut(*slot).as_mut().unwrap();
                set_custom(scroll, CONTRACT, Tag::Long(serial as i64));
                set_custom(scroll, OWNER, string(owner));
            } else {
                *self.stack_in_mut(*slot) = None;
            }
            l.preview = None;
            self.sc_save(&l);
            self.sc_notice(if verb == "accept" {
                "Quest accepted. Open the journal with J."
            } else {
                "Quest scroll discarded."
            });
            return;
        }
        let Some(i) = l.active.iter().position(|c| c.serial == serial) else {
            return;
        };
        if verb == "abandon" {
            self.sc_remove_scroll(serial);
            l.active.remove(i);
            self.sc_save(&l);
            self.sc_notice("Quest abandoned.");
            return;
        }
        if verb != "claim" {
            return;
        }
        let Some(keeper) = parts.next() else {
            return;
        };
        let p = self.player.as_ref().unwrap().position;
        let nearby: Vec<_> = self
            .mobs
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                m.species() == Species::Villager
                    && m.extra.variant2 == 17
                    && m.alive()
                    && !m.removed
                    && !m.is_baby()
                    && (m.position.x - p.x).powi(2)
                        + (m.position.y - p.y).powi(2)
                        + (m.position.z - p.z).powi(2)
                        <= 36.
            })
            .map(|(i, m)| (i, m.id))
            .collect();
        let Some((index, _)) = nearby
            .into_iter()
            .find(|(_, id)| self.sc_keeper_id(*id) == keeper)
        else {
            return;
        };
        let c = &l.active[i];
        let kind = c.kind;
        if self.sc_progress(c) < kind.target() {
            self.sc_notice("The quest criteria are not complete yet.");
            return;
        }
        if self.native_room_for("minecraft:emerald") < kind.reward() {
            self.sc_notice("Make room for the emeralds and claim again.");
            return;
        }
        if let Some(item) = kind.delivery() {
            self.native_take_items(item, kind.target());
        }
        self.sc_remove_scroll(serial);
        self.native_give_items("minecraft:emerald", kind.reward());
        self.experience.give((kind.reward() * 3) as i32);
        // Apply the raid victory effect to the acting player, without claiming a raid victory.
        let mut hero = horizon_client::effects::EffectInstance::new(
            "hero_of_the_village", horizon_client::raid::HERO_DURATION, (kind.hero_rank() - 1) as i32,
        );
        hero.visible = false;
        self.potions.effects.add(hero);
        l.active.remove(i);
        self.sc_save(&l);
        let m = &mut self.mobs[index];
        let trade = m.extra.trade.get_or_insert_default();
        trade.xp = trade.xp.saturating_add((kind.reward() * 4) as i32);
        let rank = horizon_client::merchant::LEVEL_XP
            .iter()
            .filter(|&&xp| xp <= trade.xp)
            .count() as u8;
        if rank > m.extra.variant3 {
            m.extra.variant3 = rank;
            trade.unset = true;
        }
        if let Some(level) = self.native_level(self.dim.id()) {
            self.native_level_play_sound(
                level,
                "minecraft:entity.player.levelup",
                [p.x, p.y, p.z],
                8,
                0.5,
                1.1,
            );
        }
        self.sc_notice(format!(
            "Quest #{serial} handed in. Earned {} emeralds and {} XP.",
            kind.reward(),
            kind.reward() * 3
        ));
    }
}

pub(super) const BADGES: &str = "strangely_charming:quest_badges";
#[derive(Serialize, Deserialize, Clone)]
struct Badge {
    kind: Kind,
    count: u32,
}
#[derive(Default)]
struct BadgeCache {
    requested: Option<std::time::Instant>,
    badges: Vec<Badge>,
}
thread_local! {static BADGE_CACHE:std::cell::RefCell<BadgeCache>=Default::default();}
pub(super) fn read_badges(bytes: &[u8]) {
    if bytes.len() <= 256
        && let Ok(badges) = serde_json::from_slice::<Vec<Badge>>(bytes)
        && badges.len() <= 2
    {
        BADGE_CACHE.with_borrow_mut(|c| c.badges = badges);
    }
}
pub(super) fn clear_badges() {
    BADGE_CACHE.with_borrow_mut(|c| *c = BadgeCache::default());
}
impl Game<'_> {
    fn sc_badges(&mut self) -> Vec<Badge> {
        self.sc_load()
            .active
            .iter()
            .map(|c| Badge {
                kind: c.kind,
                count: self.sc_progress(c),
            })
            .collect()
    }
    fn sc_send_badges(&mut self) {
        use horizon_protocol::packets::{configuration::CustomPayload, play::Clientbound};
        let badges = self.sc_badges();
        let who = self.native_acting_id();
        let data = serde_json::to_vec(&badges).unwrap();
        self.native_send(
            who,
            Clientbound::CustomPayload(CustomPayload {
                channel: BADGES.into(),
                data,
            }),
        );
    }
    pub(super) fn sc_inventory_badges(&mut self, batch: &mut super::HudBatch, ox: f32, oy: f32) {
        let refresh = BADGE_CACHE.with_borrow_mut(|c| {
            if c.requested.is_some_and(|at| at.elapsed().as_secs() < 1) {
                return false;
            }
            c.requested = Some(std::time::Instant::now());
            true
        });
        if refresh {
            if let Some(remote) = &self.remote {
                remote
                    .session
                    .send_custom_click("strangely_charming:quest/badges", None);
            } else {
                let badges = self.sc_badges();
                BADGE_CACHE.with_borrow_mut(|c| c.badges = badges);
            }
        }
        BADGE_CACHE.with_borrow(|c| {
            for (i, badge) in c.badges.iter().enumerate() {
                let width = 80.;
                let x = ox + 3. + i as f32 * 87.;
                let y = if oy >= 17. { oy - 16. } else { oy + 167. };
                if y + 15. > batch.height {
                    continue;
                }
                let label = match badge.kind {
                    Kind::Wheat => "Wheat",
                    Kind::Bread => "Bread",
                    Kind::Coal => "Coal",
                    Kind::Zombies => "Zombies",
                    Kind::Books => "Books",
                };
                let max = badge.kind.target();
                let n = badge.count.min(max);
                let atlas = &self.gui.atlas;
                batch.fill(
                    atlas,
                    x - 1.,
                    y - 1.,
                    width + 2.,
                    14.,
                    super::rgba(32, 25, 29, 235),
                );
                batch.text(
                    atlas,
                    &format!("{label} {n}/{max}"),
                    x + 2.,
                    y,
                    super::rgba(255, 255, 255, 255),
                    false,
                );
                batch.fill(
                    atlas,
                    x + 1.,
                    y + 10.,
                    width - 2.,
                    2.,
                    super::rgba(80, 75, 73, 255),
                );
                batch.fill(
                    atlas,
                    x + 1.,
                    y + 10.,
                    (width - 2.) * n as f32 / max as f32,
                    2.,
                    super::rgba(110, 208, 137, 255),
                );
            }
        });
    }
}
