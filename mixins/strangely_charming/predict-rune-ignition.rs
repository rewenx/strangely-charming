fn hook(
    game: &crate::game::Game<'_>,
    hit: &super::Hit,
    held: Option<&ItemStack>,
) -> core::ops::ControlFlow<(InteractionResult, DryUse)> {
    if game.sc_igniter_target(hit.pos, held) {
        return core::ops::ControlFlow::Break((InteractionResult::Success, DryUse::default()));
    }
    core::ops::ControlFlow::Continue(())
}
