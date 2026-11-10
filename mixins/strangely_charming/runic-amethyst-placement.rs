fn hook(
    level: &crate::game::GameLevel<'_>,
    state: BlockStateId,
    context: &PlacementContext,
    answer: &mut Option<Option<BlockStateId>>,
) {
    if answer.is_none()
        && level
            .native_state_block(state)
            .is_some_and(|id| id.starts_with("strangely_charming:runic_amethyst_"))
    {
        let facing = crate::game::sc_runes::manual_facing(context.yaw, context.pitch);
        if let Some(oriented) = level.native_state_with_property(state, "facing", facing) {
            *answer = Some(Some(oriented));
        }
    }
}
