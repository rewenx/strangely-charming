fn hook(game: &mut crate::game::Game<'_>, p: &NativeServerPlayer, cmd: &str, ran: &mut bool) {
    if !*ran
        && p.permission >= 2
        && let Some(msg) = game
            .sc_rune_command(cmd, Some(p.uuid))
            .or_else(|| game.sc_grave_command(cmd, Some(p.position.map(|n| n.floor() as i32))))
    {
        game.native_tell(p.uuid, horizon_client::text::Text::literal(msg));
        *ran = true;
    }
}
