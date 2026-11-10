fn hook(game: &crate::game::Game<'_>, batch: &mut HudBatch, front: bool) {
    if !front {
        game.sc_draw_cycle(batch);
    }
}
