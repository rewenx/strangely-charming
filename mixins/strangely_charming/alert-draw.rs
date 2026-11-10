fn hook(game: &crate::game::Game<'_>, batch: &mut HudBatch, hidden: bool) {
    game.sc_camera_overlay(batch);
    if !hidden && game.title.is_none() {
        game.sc_draw_held_page(batch);
        game.sc_draw_grave_guide(batch);
        game.sc_rune_label(batch);
        game.sc_draw_alert(batch);
        game.sc_draw_placement_prompt(batch);
    }
}
