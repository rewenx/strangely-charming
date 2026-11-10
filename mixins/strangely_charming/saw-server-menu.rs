fn hook(
    value: Box<Menu>,
    game: &crate::game::Game<'_>,
    station: &Station,
    host: &LanHost,
    conn: u32,
) -> Box<Menu> {
    let mut menu = value;
    menu.evanders_set_table_saw(
        game.block_name(station.pos).as_ref() == "strangely_charming:woodcutter",
    );
    if menu.kind() == Kind::Stonecutter
        && !menu.evanders_is_table_saw()
        && let Some(owner) = host.hub.guest(conn).map(|g| g.uuid)
    {
        menu.sc_set_cut_glyphs(game.sc_owner_mask(owner));
    }
    menu
}
