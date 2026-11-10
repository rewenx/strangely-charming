fn hook(game: &mut crate::game::Game<'_>, local: (f32, f32)) -> core::ops::ControlFlow<bool> {
    if game.sc_cycle_click(local) {
        core::ops::ControlFlow::Break(true)
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
