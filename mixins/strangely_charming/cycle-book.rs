fn hook(game: &mut crate::game::Game<'_>, _ret: ()) {
    let Some(id) = game
        .station
        .as_ref()
        .and_then(|s| s.merchant.as_ref())
        .and_then(|m| m.mob)
    else {
        return;
    };
    if game.sc_traded(id) {
        return;
    }
    if let Some(m) = game.station.as_mut().and_then(|s| s.merchant.as_mut()) {
        crate::game::stations::sc_trade_cycle::sort_books(&mut m.session);
    }
}
