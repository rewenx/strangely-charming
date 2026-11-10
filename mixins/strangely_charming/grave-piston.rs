fn hook(g: &GameLevel<'_>, push: &[BlockPos], destroy: &[BlockPos], cancel: &mut bool) {
    *cancel |= push.iter().chain(destroy).any(|p| {
        crate::game::sc_graves::is_marker(&g.facts.get(g.world.block(p.x, p.y, p.z)).name)
    });
}
