fn hook(game: &crate::game::Game<'_>, scroll: [f64; 2], canceled: &mut bool) {
    *canceled |= game.sc_journal_scroll(scroll[1]);
}
