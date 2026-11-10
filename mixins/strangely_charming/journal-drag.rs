fn hook(game: &crate::game::Game<'_>, canceled: &mut bool) {
    *canceled |= game.sc_journal_drag();
}
