fn hook(game: &mut crate::game::Game<'_>) {
    game.sc_repair_tick();
    game.sc_rune_tick();
    game.sc_grave_tick();
    game.sc_check_ready();
}
