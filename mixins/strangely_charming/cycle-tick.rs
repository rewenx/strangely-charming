fn hook(game: &mut crate::game::Game<'_>) {
    game.sc_watch_grave();
    game.sc_cycle_tick();
}
