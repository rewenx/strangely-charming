fn hook(game: &crate::game::Game<'_>, key: KeyCode) {
    if let Some(name) = keybinds::key_name(key) {
        game.sc_journal_release(name);
    }
}
