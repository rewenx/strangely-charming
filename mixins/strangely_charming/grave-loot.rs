fn hook(
    game: &mut crate::game::Game<'_>,
    context: &NativeLootDrops<'_>,
    drops: &mut Vec<ItemStack>,
    _ret: (),
) {
    game.sc_capture_drops(context, drops);
}
