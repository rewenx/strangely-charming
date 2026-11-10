fn hook(game: &mut crate::game::Game<'_>, pos: [i32; 3]) -> core::ops::ControlFlow<f32> {
    if !game.creative && game.sc_rune_at(pos) && !game.sc_has_hammer() {
        return core::ops::ControlFlow::Break(0.0);
    }
    core::ops::ControlFlow::Continue(())
}
