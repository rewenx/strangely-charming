fn hook(game: &crate::game::Game<'_>) -> core::ops::ControlFlow<Option<PlayerView>> {
    if game.sc_shielded() {
        core::ops::ControlFlow::Break(None)
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
