fn hook(game: &crate::game::Game<'_>, raw: KeyCode) -> core::ops::ControlFlow<Result<()>> {
    if raw == KeyCode::Escape && (game.sc_cancel_preview() || game.sc_camera_cancel() || game.sc_cancel_waypoint()) {
        return core::ops::ControlFlow::Break(Ok(()));
    }
    core::ops::ControlFlow::Continue(())
}
