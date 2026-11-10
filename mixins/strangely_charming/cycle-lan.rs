use super::{C, Game, GuestMerchant, LanHost};
use crate::game::stations::sc_trade_cycle::CAP;
use horizon_protocol::packets::configuration::CustomPayload;
impl Game<'_> {
    pub(in crate::game) fn sc_lan_cycle_cap(
        &self,
        host: &mut LanHost,
        conn: u32,
        gm: &GuestMerchant,
    ) {
        if !horizon_mods::evanders_config::enabled("cycle_villager_trades") {
            return;
        }
        let mut data = gm.window.to_be_bytes().to_vec();
        data.push(u8::from(self.sc_cycle_index(&gm.merchant).is_some()));
        host.hub.send_to(
            conn,
            &C::CustomPayload(CustomPayload {
                channel: CAP.into(),
                data,
            }),
        );
    }
    pub(in crate::game) fn sc_lan_cycle(&mut self, host: &mut LanHost, conn: u32, window: i32) {
        let Some(gm) = host.play.get(&conn).and_then(|p| p.merchant.as_ref()) else {
            return;
        };
        let Some(g) = host
            .hub
            .guests
            .iter()
            .find(|g| g.conn == conn && g.placed && !g.dead && !g.spectator() && g.health > 0.)
        else {
            return;
        };
        if gm.window != window
            || gm.slots.iter().any(Option::is_some)
            || gm.merchant.session.active.is_some()
            || !gm
                .merchant
                .mob
                .is_some_and(|id| self.lan_merchant_valid(g, id))
        {
            return;
        }
        let Some(mut gm) = host.play.get_mut(&conn).and_then(|p| p.merchant.take()) else {
            return;
        };
        if self.sc_cycle_offers(&mut gm.merchant) {
            self.lan_send_offers(host, conn, &mut gm, true);
            self.lan_send_merchant_content(host, conn, &mut gm);
        }
        host.play.entry(conn).or_default().merchant = Some(gm);
    }
}
