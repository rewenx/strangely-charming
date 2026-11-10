fn hook(game: &crate::game::Game<'_>, out: &mut WorldPrimitives) {
    game.sc_draw_preview(out);
}
