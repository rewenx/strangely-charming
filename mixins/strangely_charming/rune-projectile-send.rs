fn hook(_ret: (), g: &crate::game::Game<'_>, hub: &mut Hub, states: &mut States, after: u32) {
    let mut pearls = g
        .creatures
        .projectiles
        .iter()
        .filter(|p| p.id > after && crate::game::sc_runes::is_rune_pearl(p))
        .peekable();
    if pearls.peek().is_none() {
        return;
    }
    let stacks: std::collections::HashMap<_, _> = pearls.map(|p| (p.id as i32, &p.item)).collect();
    for state in states.as_mut_slice() {
        if let Some(stack) = stacks.get(&state.id) {
            state
                .meta
                .push((8, serializer::ITEM_STACK, Value::Item(hub.stack(stack))));
        }
    }
}
