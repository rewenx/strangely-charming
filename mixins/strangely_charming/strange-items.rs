use super::Game;
use horizon_client::{entities::Experience, items::ItemStack};
use horizon_world::nbt::{Compound, Tag};

pub(super) const TOTEM: &str = "strangely_charming:totem_of_enchanting";
const POINTS: &str = "strangely_charming:experience";

pub(super) fn custom<'a>(stack: &'a ItemStack, key: &str) -> Option<&'a Tag> {
    stack.data()?.other("minecraft:custom_data")?.get(key)
}
pub(super) fn set_custom(stack: &mut ItemStack, key: &str, value: Tag) {
    let mut tag = stack
        .data()
        .and_then(|d| d.other("minecraft:custom_data"))
        .cloned()
        .unwrap_or_else(|| Tag::Compound(Compound::new()));
    if let Tag::Compound(fields) = &mut tag {
        fields.insert(key.into(), value);
        stack
            .data_mut()
            .set_other("minecraft:custom_data", Some(tag));
    }
}
fn update_balance(stack: &mut ItemStack, old: i32, new: i32) {
    set_custom(stack, POINTS, Tag::Int(new));
    let mut lore = match stack.data().and_then(|d| d.other("minecraft:lore")) {
        Some(Tag::List(lines)) => lines.clone(),
        _ => Vec::new(),
    };
    let previous = Tag::String(format!("Stored XP: {old}").as_str().into());
    if old > 0
        && let Some(i) = lore.iter().position(|line| line == &previous)
    {
        lore.remove(i);
    }
    if new > 0 {
        lore.push(Tag::String(format!("Stored XP: {new}").as_str().into()));
    }
    stack.data_mut().set_other(
        "minecraft:lore",
        (!lore.is_empty()).then_some(Tag::List(lore)),
    );
}
fn base(level: i64) -> i64 {
    match level {
        0..=16 => level * (level + 6),
        17..=31 => (5 * level * level - 81 * level + 720) / 2,
        _ => (9 * level * level - 325 * level + 4440) / 2,
    }
}
fn points(xp: Experience) -> Option<i32> {
    let level = xp.level;
    if !(0..=21864).contains(&level) || !xp.progress.is_finite() {
        return None;
    }
    let partial = (f64::from(xp.progress) * f64::from(Experience::needed(level))).round() as i64;
    i32::try_from(base(i64::from(level)) + partial.max(0)).ok()
}
fn from_points(points: i32) -> Experience {
    let mut lo = 0i64;
    let mut hi = 21865;
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if base(mid) <= i64::from(points) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Experience {
        level: lo as i32,
        total: points,
        progress: (i64::from(points) - base(lo)) as f32 / Experience::needed(lo as i32) as f32,
    }
}
impl Game<'_> {
    pub(super) fn sc_rune_at(&self, pos: [i32; 3]) -> bool {
        self.block_name(pos)
            .starts_with("strangely_charming:runestone_")
    }
    pub(super) fn sc_has_hammer(&self) -> bool {
        horizon_mods::evanders_config::enabled("runebreaker")
            && self
                .held()
                .is_some_and(|s| s.item.as_ref() == "strangely_charming:runebreaker")
    }

    pub(super) fn sc_use_item(&mut self, stack: &ItemStack) -> bool {
        if stack.count == 0 {
            return false;
        }
        if stack.item.as_ref() == super::sc_runes::PEARL && super::sc_runes::enabled() {
            return self.sc_throw_rune(stack);
        }
        if stack.item.as_ref() == super::sc_field_items::KNOWLEDGE
            && horizon_mods::evanders_config::enabled("knowledge_stone")
        {
            if self.remote.is_none() {
                if self
                    .pick()
                    .is_some_and(|hit| self.sc_igniter_target(hit.pos, Some(stack)))
                {
                    return true;
                }
                if !self.sc_unlock_recipes() {
                    return true;
                }
                let held = self.held_mut();
                if let Some(s) = held {
                    s.count -= 1;
                    if s.count == 0 {
                        *held = None;
                    }
                }
            }
            return true;
        }
        if stack.item.as_ref() == super::sc_field_items::SLIME
            && horizon_mods::evanders_config::enabled("slime_stone")
        {
            if self.remote.is_none() {
                self.sc_use_slime();
            }
            return true;
        }
        if self.sc_travel_use(stack) {
            return true;
        }
        if horizon_mods::evanders_config::enabled("scrollkeepers")
            && let Some(kind) = super::sc_quests::Kind::item(&stack.item)
        {
            self.sc_scroll(stack, kind);
            return true;
        }
        if stack.item.as_ref() != TOTEM
            || !horizon_mods::evanders_config::enabled("enchanting_totems")
        {
            return false;
        }
        let stored = match custom(stack, POINTS) {
            Some(Tag::Int(n)) if *n >= 0 => *n,
            None => 0,
            _ => {
                self.sc_notice("This totem's stored XP could not be read.");
                return true;
            }
        };
        let Some(balance) = points(self.experience) else {
            self.sc_notice("Your XP exceeds the totem's capacity.");
            return true;
        };
        let Some(sum) = stored.checked_add(balance) else {
            self.sc_notice("There is too much XP to combine.");
            return true;
        };
        if self.native_sneaking() {
            if balance == 0 {
                self.sc_notice(format!(
                    "Totem holds {stored} XP. You have no XP to deposit."
                ));
                return true;
            }
            let Some(held) = self.held_mut().as_mut() else {
                return true;
            };
            update_balance(held, stored, sum);
            self.experience = Experience::default();
            self.sc_notice(format!("Deposited {balance} XP. Totem holds {sum} XP."));
        } else if stored == 0 {
            self.sc_notice("Sneak and use this totem to store your XP.");
        } else {
            let Some(held) = self.held_mut().as_mut() else {
                return true;
            };
            update_balance(held, stored, 0);
            self.experience = from_points(sum);
            self.sc_notice(format!("The totem returned {stored} XP."));
        }
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bank_round_trips_spendable_points() {
        for n in (0..20000).chain([999_999, i32::MAX - 1, i32::MAX]) {
            assert_eq!(points(from_points(n)), Some(n));
        }
        let mut spent = from_points(353);
        spent.total = 2500;
        assert_eq!(points(spent), Some(353));
    }
}
