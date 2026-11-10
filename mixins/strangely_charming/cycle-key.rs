fn hook(game: &crate::game::Game<'_>, key: KeyCode, repeat: bool, value: bool) -> bool {
    value
        || game.sc_journal_key(key)
        || keybinds::key_name(key).is_some_and(|name| {
            game.sc_share_input(name, repeat) || game.sc_lookup_press(name, repeat) || game.sc_cycle_input(name, repeat)
        })
}
