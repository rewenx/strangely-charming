fn hook(game: &mut crate::game::Game<'_>, session: &merchant::Session, mob: Option<u32>) {
    if let Some(id) = mob
        && session.villager
        && session.xp == 0
        && session
            .events
            .iter()
            .any(|e| matches!(e, merchant::Event::Traded(..)))
    {
        game.sc_mark_trade(id);
    }
}
