use super::Game;
use horizon_client::{items::ItemStack, mobs::Species};
use horizon_world::nbt::{Compound, Tag};
const APPLE: &str = "strangely_charming:poison_apple";
use horizon_client::mobs::strangely_charming_poison as poison;

pub(super) fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if pressed
        && game.title.is_none()
        && (game.native_playing()
            || game.screen == super::Screen::Inventory
            || game.creative_inventory_open())
    {
        if horizon_mods::evanders_config::enabled("scrollkeepers")
            && key==game.native_mod_key("strangely_charming","scrollkeepers","key.keyboard.j") {
            if let Some(remote)=&game.remote {
                remote.session.send_custom_click("strangely_charming:quest/journal",None);
            } else {game.sc_journal(None);}
            return;
        }
        let portable = game.remote_menu_id() == 0
            && key == game.native_mod_key("strangely_charming", "portable_crafting", "key.keyboard.v")
            && horizon_mods::evanders_config::enabled("portable_crafting");
        let tidy = key
            == game.native_mod_key("strangely_charming", "inventory_tidying", "key.mouse.middle")
            && horizon_mods::evanders_config::enabled("inventory_tidying");
        if (portable || tidy) && game.remote.is_some() {
            let mut data = vec![if portable { 1 } else { 2 }];
            data.extend_from_slice(&game.remote_menu_id().to_be_bytes());
            game.send(horizon_protocol::packets::play::Serverbound::CustomPayload(
                horizon_protocol::packets::configuration::CustomPayload {
                    channel: ACTION_CHANNEL.into(),
                    data,
                },
            ));
            return;
        }
    }
    if pressed && game.native_playing() && game.remote.is_none() {
        if horizon_mods::evanders_config::enabled("portable_crafting")
            && key == game.native_mod_key("strangely_charming", "portable_crafting", "key.keyboard.v")
            && game
                .inventory
                .iter()
                .flatten()
                .chain(game.offhand.iter())
                .any(|s| s.item.as_ref() == "minecraft:crafting_table")
        {
            game.craft_size = 3;
            game.craft_table = None;
            game.screen = super::Screen::Inventory;
            game.capture(false);
            return;
        }
    }
    if pressed
        && game.remote.is_none()
        && game.title.is_none()
        && (game.native_playing()
            || game.screen == super::Screen::Inventory
            || game.creative_inventory_open())
        && horizon_mods::evanders_config::enabled("inventory_tidying")
        && key == game.native_mod_key("strangely_charming", "inventory_tidying", "key.mouse.middle")
    {
        let container = game
            .container
            .filter(|&(x, y, z)| {
                let block = game.block_name([x, y, z]);
                matches!(
                    block.as_ref(),
                    "minecraft:chest"
                        | "minecraft:trapped_chest"
                        | "minecraft:barrel"
                        | "minecraft:ender_chest"
                ) || block.ends_with("shulker_box")
            })
            .is_some();
        let range = if container {
            0..game.container_size()
        } else {
            9..game.inventory.len()
        };
        let mut stacks: Vec<_> = range
            .clone()
            .map(|slot| {
                if container {
                    game.container_slot_mut(slot).take()
                } else {
                    game.inventory[slot].take()
                }
            })
            .collect();
        compact(&mut stacks, |s| game.max_stack(s));
        for (slot, stack) in range.zip(stacks) {
            if container {
                *game.container_slot_mut(slot) = stack;
            } else {
                game.inventory[slot] = stack;
            }
        }
        return;
    }
    if !horizon_mods::evanders_config::enabled("fullbright")
        || !pressed
        || !game.native_playing()
        || key != game.native_mod_key("strangely_charming", "fullbright", "key.keyboard.g")
    {
        return;
    }
    game.settings.extras.fullbright = !game.settings.extras.fullbright;
    let on = game.settings.extras.fullbright;
    if let Some(renderer) = game.renderer.as_mut() {
        renderer.fullbright = on;
    }
    game.settings.save(&Game::options_path());
    game.sc_notice(if on {
        "Fullbright: on"
    } else {
        "Fullbright: off"
    });
}

pub(super) fn interact(
    game: &mut Game<'_>,
    entity: i32,
    stack: Option<&ItemStack>,
    claimed: &mut bool,
) {
    if !horizon_mods::evanders_config::enabled("poison_apple")
        || *claimed
        || stack.is_none_or(|s| s.item.as_ref() != APPLE || s.count == 0)
    {
        return;
    }
    let Some(i) = game.mobs.iter().position(|m| m.id == entity as u32) else {
        return;
    };
    let m = &game.mobs[i];
    if m.species() != Species::Villager {
        return;
    }
    *claimed = true;
    if !m.alive() || m.removed {
        return;
    }
    if !poison::eligible(m) {
        game.sc_notice("Poison apples cannot be used on child villagers or nitwits.");
        return;
    }
    game.ensure_offers(i);
    let old = game.mobs[i].clone();
    let at = [old.position.x, old.position.y, old.position.z];
    // Spawn first: failure must leave the original villager and apple intact.
    let Some(id) = game.spawn_mob("minecraft:zombie_villager", at, true) else {
        return;
    };
    let Some(z) = game.mobs.iter_mut().find(|m| m.id == id) else {
        return;
    };
    super::zombie_villagers::convert_to_zombie_villager(&old, z);
    let state = poison::infect(z, old.no_ai);
    let raw = game
        .persist
        .raw
        .entry(id)
        .or_insert_with(|| Tag::Compound(Compound::new()));
    if let Tag::Compound(raw) = raw {
        raw.insert(poison::STATE.into(), state);
    }
    if let Some(v) = game.mobs.iter_mut().find(|m| m.id == old.id) {
        v.removed = true;
    }
    game.consume_held();
    game.level_event_at(
        1026,
        [
            at[0].floor() as i32,
            at[1].floor() as i32,
            at[2].floor() as i32,
        ],
        0,
    );
}

// Stored in the entity's raw NBT so immunity and remaining stun survive reloads.
// The kind is Arc-shared normally; make_mut prevents affecting other zombies.
pub(super) fn level_tick(game: &mut Game<'_>) {
    if game.remote.is_some() {
        return;
    }
    let ticking = game.entity_ticking_area();
    for m in &mut game.mobs {
        if let Some(raw) = game.persist.raw.get_mut(&m.id) {
            if m.kind.flags.burns_in_day && matches!(raw.get(super::sc_graves::TEST_MOB), Some(Tag::Byte(1))) {
                std::sync::Arc::make_mut(&mut m.kind).flags.burns_in_day = false;
            }
            poison::before_tick(
                m,
                raw,
                ticking(m.position.x.floor() as i32, m.position.z.floor() as i32),
            );
        }
    }
}

pub(super) const ACTION_CHANNEL: &str = "strangely_charming:inventory_action";
pub(super) const PORTABLE_TABLE: [i32; 3] = [0, i32::MIN, 0];
pub(super) fn carries_table(g: &horizon_client::lan::guest::Guest) -> bool {
    g.inventory
        .iter()
        .flatten()
        .chain(g.offhand.iter())
        .any(|s| s.item.as_ref() == "minecraft:crafting_table" && s.count > 0)
}
pub(super) fn compact(slots: &mut [Option<ItemStack>], limit: impl Fn(&ItemStack) -> u32) {
    let mut result: Vec<ItemStack> = Vec::new();
    for mut stack in slots.iter_mut().filter_map(Option::take) {
        let max = stack
            .data()
            .and_then(|d| match d.other("minecraft:max_stack_size") {
                Some(Tag::Int(n)) if (1..=99).contains(n) => Some(*n as u32),
                _ => None,
            })
            .unwrap_or_else(|| limit(&stack));
        for other in &mut result {
            if !other.same_item_same_components(&stack) {
                continue;
            }
            let moved = stack.count.min(max.saturating_sub(other.count));
            other.count += moved;
            stack.count -= moved;
        }
        if stack.count > 0 {
            result.push(stack);
        }
    }
    result.sort_by(|a, b| a.item.cmp(&b.item));
    for (dest, stack) in slots.iter_mut().zip(result) {
        *dest = Some(stack);
    }
}
