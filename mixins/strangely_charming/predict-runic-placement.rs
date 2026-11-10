fn hook(
    mut predicted: Option<DryPlace>,
    game: &crate::game::Game<'_>,
    stack: &ItemStack,
) -> Option<DryPlace> {
    if stack.item.starts_with("strangely_charming:runic_amethyst_")
        && let Some((placed, changes)) = &mut predicted
        && let Some((_, state)) = changes.iter_mut().find(|(pos, _)| pos == placed)
        && let Some(oriented) = game.with_props(
            *state,
            &[(
                "facing",
                crate::game::sc_runes::manual_facing(game.yaw, game.pitch),
            )],
        )
    {
        *state = oriented;
    }
    predicted
}
