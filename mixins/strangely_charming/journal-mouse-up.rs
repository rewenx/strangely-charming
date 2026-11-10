fn hook(game: &crate::game::Game<'_>, button: i32) {
    game.sc_journal_release(&match button {
        0 => "key.mouse.left".into(),
        1 => "key.mouse.right".into(),
        2 => "key.mouse.middle".into(),
        n => format!("key.mouse.{}", n + 1),
    });
}
