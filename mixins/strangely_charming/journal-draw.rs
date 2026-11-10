fn hook(game: &crate::game::Game<'_>, batch: &mut HudBatch) {
    game.sc_draw_journal(batch);
}
