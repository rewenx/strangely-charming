fn hook(game: &mut crate::game::Game<'_>, cmd: &str, ran: &mut bool) {
    if !*ran
        && let Some(msg) = game
            .sc_rune_command(cmd, None)
            .or_else(|| game.sc_grave_command(cmd, None))
    {
        log::info!("{}", msg);
        game.native_feedback(horizon_client::text::Text::literal(msg));
        *ran = true;
    }
}
