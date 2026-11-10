fn hook(game: &mut crate::game::Game<'_>, stack: &ItemStack) -> core::ops::ControlFlow<bool> {
    if game.remote.is_some()
        && ((crate::game::sc_travel::handles(&stack.item)
            && (stack.item.as_ref() != crate::game::sc_travel::BOOK || !game.native_sneaking()))
            || (stack.item.as_ref() == crate::game::sc_runes::PEARL
                && crate::game::sc_runes::enabled())
            || (stack.item.as_ref() == crate::game::sc_field_items::SLIME
                && horizon_mods::evanders_config::enabled("slime_stone"))
            || (stack.item.as_ref() == crate::game::sc_field_items::KNOWLEDGE
                && horizon_mods::evanders_config::enabled("knowledge_stone"))
            || (stack.item.as_ref() == crate::game::sc_items::TOTEM
                && horizon_mods::evanders_config::enabled("enchanting_totems"))
            || (crate::game::sc_quests::Kind::item(&stack.item).is_some()
                && horizon_mods::evanders_config::enabled("scrollkeepers")))
    {
        return core::ops::ControlFlow::Break(true);
    }
    if stack.item.as_ref() != "strangely_charming:raid_horn" {
        return core::ops::ControlFlow::Continue(());
    }
    if !horizon_mods::evanders_config::enabled("raid_horns") {
        return core::ops::ControlFlow::Break(false);
    }
    if game.remote.is_some() {
        game.evanders_play_horn(stack);
    } else {
        game.use_instrument(stack);
        if let Some(p) = game.player.as_ref()
            && let Some(id) = game.raiding_nearby([
                p.position.x.floor() as i32,
                p.position.y.floor() as i32,
                p.position.z.floor() as i32,
            ])
        {
            game.raid_command_stop(id);
        }
    }
    core::ops::ControlFlow::Break(true)
}
