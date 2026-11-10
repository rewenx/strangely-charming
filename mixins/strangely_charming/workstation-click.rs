fn hook(game: &mut crate::game::Game<'_>, pos: [i32; 3], canceled: &mut bool) {
    if !*canceled && (game.sc_restore_click(pos) || game.sc_ignite(pos)) {
        *canceled = true;
        return;
    }
    if !*canceled && game.sc_grave_use(pos) {
        *canceled = true;
        return;
    }
    if game.block_name(pos).as_ref() == "strangely_charming:writing_desk" && !game.native_sneaking()
    {
        *canceled = true;
        return;
    }
    if *canceled
        || game.keys.contains(&winit::keyboard::KeyCode::ShiftLeft)
            && (game.held().is_some() || game.offhand.is_some())
    {
        return;
    }
    let feature = match game.block_name(pos).as_ref() {
        "strangely_charming:kiln" => "kilns",
        "strangely_charming:woodcutter" => "lumberjacks",
        _ => return,
    };
    if horizon_mods::evanders_config::enabled(feature) {
        game.open_menu(pos);
        *canceled = true;
    }
}
