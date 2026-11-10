fn hook(level: &mut crate::game::GameLevel<'_>, pos: BlockPos, state: BlockStateId) {
    if !level.native_authoritative || level.debug && !level.client_side {
        return;
    }
    let old = level.world.block(pos.x, pos.y, pos.z);
    if old != state
        && level
            .facts
            .get(old)
            .name
            .starts_with("strangely_charming:runestone_")
    {
        // set_raw can only reject a debug world, missing chunk or out-of-range Y.
        if level
            .world
            .chunks
            .get(&(pos.x >> 4, pos.z >> 4))
            .is_some_and(|c| c.section_index(pos.y).is_some())
        {
            crate::game::sc_restoration::mark_socket(level.world, [pos.x, pos.y, pos.z], true);
        }
    }
}
