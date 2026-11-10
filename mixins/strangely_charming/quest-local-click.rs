fn hook(
    game: &mut crate::game::Game<'_>,
    click: &Option<Click>,
) -> core::ops::ControlFlow<Result<()>> {
    if game.remote.is_none() && let Some(Click::Custom{id,..})=click && let Some(a)=id.strip_prefix(crate::game::sc_restoration::CHANNEL){game.sc_restore_answer(a);}
    if game.remote.is_none()
        && let Some(Click::Custom { id, .. }) = click
        && let Some(action) = id.strip_prefix("strangely_charming:quest/")
    {
        game.sc_quest_action(action);
    }
    core::ops::ControlFlow::Continue(())
}
