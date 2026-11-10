fn hook(g: &crate::game::Game<'_>, blocks: &mut Vec<[i32; 3]>) {
    blocks.retain(|p| !crate::game::sc_graves::is_marker(&g.block_name(*p)));
}
