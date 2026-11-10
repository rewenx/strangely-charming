fn hook(game: &crate::game::Game<'_>, c: char, canceled: &mut bool) {
    *canceled |= game.sc_journal_char(c);
}
