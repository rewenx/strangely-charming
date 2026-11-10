fn hook(
    game: &crate::game::Game<'_>,
    host: &mut LanHost,
    conn: u32,
    gm: &GuestMerchant,
    force: bool,
) {
    if force {
        game.sc_lan_cycle_cap(host, conn, gm);
    }
}
