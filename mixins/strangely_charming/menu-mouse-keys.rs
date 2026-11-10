// Horizon sends native mouse-key events during play. Its inventory screens
// route mouse buttons through screen input instead. Honor the same saved
// mappings there without forwarding clicks from text fields or options.
fn hook(game: &mut crate::game::Game<'_>, button: MouseButton) -> core::ops::ControlFlow<bool> {
    if game.title.is_none()
        && (game.screen == super::Screen::Inventory || game.creative_inventory_open())
    {
        let name = keybinds::mouse_name(button);
        if [
            ("inventory_tidying", "key.mouse.middle"),
            ("scrollkeepers", "key.keyboard.j"),
        ]
        .iter()
        .any(|&(id, default)| {
            horizon_mods::evanders_config::enabled(id)
                && name == game.native_mod_key("strangely_charming", id, default)
        }) {
            crate::game::strangely_charming::key(game, &name, true);
            return core::ops::ControlFlow::Break(true);
        }
    }
    core::ops::ControlFlow::Continue(())
}
