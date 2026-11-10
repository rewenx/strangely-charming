fn hook(game: &mut crate::game::Game<'_>, start: bool) -> core::ops::ControlFlow<()> {
    if let Some(hit) = game.pick()
        && crate::game::sc_graves::is_marker(&game.block_name(hit.pos))
        && game.sc_client_block_grave(hit.pos, start)
    {
        return core::ops::ControlFlow::Break(());
    }
    if !game.creative && !game.sc_has_hammer() && game.pick().is_some_and(|hit| game.sc_rune_at(hit.pos)) {
        game.sc_warn_runebreaker();
        game.stop_remote_mining();
        game.mining = None;
        return core::ops::ControlFlow::Break(());
    }
    core::ops::ControlFlow::Continue(())
}
