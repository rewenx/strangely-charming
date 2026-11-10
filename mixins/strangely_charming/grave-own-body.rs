fn hook(invisible: bool, game: &crate::game::Game<'_>) -> bool {
    invisible || (game.remote.is_none() && game.sc_shielded())
}
