use crate::game::{Game, Screen, trading::MerchantState};
use horizon_client::{hud::HudBatch, merchant::strangely_charming_cycle::fresh, mobs::Species};
use horizon_protocol::packets::{configuration::CustomPayload, play::Serverbound};
use horizon_world::nbt::{Compound, Tag};

pub(in crate::game) const CAP: &str = "strangely_charming:trade_cycle";
const TRADED: &str = "strangely_charming:completed_trade";
const BUTTON: (f32, f32, f32, f32) = (62., 55., 74., 15.);

fn hit((x, y): (f32, f32)) -> bool {
    let (bx, by, w, h) = BUTTON;
    (bx..bx + w).contains(&x) && (by..by + h).contains(&y)
}

impl Game<'_> {
    pub(in crate::game) fn sc_mark_trade(&mut self, id: u32) {
        // Native XP permanently locks normal trades. Remember zero-XP trades as
        // well: restocking can later clear all per-offer use counts.
        if let Tag::Compound(raw) = self
            .persist
            .raw
            .entry(id)
            .or_insert_with(|| Tag::Compound(Compound::new()))
        {
            raw.insert(TRADED.into(), Tag::Byte(1));
        }
    }
    pub(in crate::game) fn sc_traded(&self, id: u32) -> bool {
        self.persist.raw.get(&id).and_then(|r| r.get(TRADED)) == Some(&Tag::Byte(1))
    }
    pub(in crate::game) fn sc_cycle_index(&self, m: &MerchantState) -> Option<usize> {
        if !horizon_mods::evanders_config::enabled("cycle_villager_trades") || !fresh(&m.session) {
            return None;
        }
        let id = m.mob?;
        if self.sc_traded(id) {
            return None;
        }
        self.mobs.iter().position(|v| {
            v.id == id
                && v.alive()
                && !v.removed
                && !v.is_baby()
                && v.species() == Species::Villager
                && v.extra.flag2
                && v.extra.variant3 <= 1
                && v.extra
                    .trade
                    .as_deref()
                    .is_some_and(|t| t.xp == 0 && t.offers.iter().all(|o| o.uses == 0))
        })
    }
    pub(in crate::game) fn sc_cycle_offers(&mut self, m: &mut MerchantState) -> bool {
        let Some(i) = self.sc_cycle_index(m) else {
            return false;
        };
        let session = &mut m.session;
        // Other registered jobs and data packs get the same first-use path as
        // normal generation. Leave profession, workstation, reputation and XP alone.
        let Some(t) = self.mobs[i].extra.trade.as_deref_mut() else {
            return false;
        };
        let old = std::mem::take(&mut t.offers);
        let was_unset = std::mem::replace(&mut t.unset, true);
        // Packs can share a random sequence between novice and later tiers.
        // Return any session loans while generating, then borrow their updated states.
        let loans: Vec<_> = std::mem::take(&mut session.sequences)
            .into_iter()
            .map(|(id, r)| {
                self.rand_sequences.give_back(id.clone(), r);
                id
            })
            .collect();
        self.ensure_offers(i);
        for id in loans {
            let r = self.rand_sequences.lend(&id, self.level.seed);
            session.sequences.insert(id, r);
        }
        if self.mobs[i].extra.variant2 == 17 {
            self.sc_stamp_scrolls(i);
        }
        let t = self.mobs[i].extra.trade.get_or_insert_default();
        if t.offers.is_empty() {
            t.offers = old;
            t.unset = was_unset;
            return false;
        }
        session.offers = t.offers.clone();
        sort_books(session);
        session.update_special_prices();
        reset(m);
        true
    }
    fn sc_cycle_ready(&self) -> bool {
        if self.screen != Screen::Inventory || self.title.is_some() {
            return false;
        }
        let Some(st) = self.station.as_ref() else {
            return false;
        };
        let Some(m) = st.merchant.as_deref() else {
            return false;
        };
        if st.inputs.iter().any(Option::is_some)
            || st.result.is_some()
            || m.session.active.is_some()
        {
            return false;
        }
        if self.remote.is_some() {
            horizon_mods::evanders_config::enabled("cycle_villager_trades")
                && *st.__horizon_mixins_strangely_charming_cycle_cap()
                && fresh(&m.session)
        } else {
            self.sc_cycle_index(m).is_some()
        }
    }
    pub(in crate::game) fn sc_receive_cycle_cap(&mut self, data: &[u8]) {
        let [a, b, c, d, flag] = *data else { return };
        let window = i32::from_be_bytes([a, b, c, d]);
        if window <= 0 || window != self.remote_menu_id() || flag > 1 {
            return;
        }
        if let Some(st) = self.station.as_mut()
            && let Some(m) = st.merchant.as_mut()
        {
            reset(m);
            *st.__horizon_mixins_strangely_charming_cycle_cap_mut() = flag == 1;
        }
    }
    pub(in crate::game) fn sc_cycle_input(&self, key: &str, repeat: bool) -> bool {
        if !self.sc_cycle_ready()
            || key
                != self.native_mod_key(
                    "strangely_charming",
                    "cycle_villager_trades",
                    "key.keyboard.c",
                )
        {
            return false;
        }
        if !repeat && let Some(st) = self.station.as_ref() {
            st.__horizon_mixins_strangely_charming_cycle_pending()
                .set(true);
        }
        true
    }
    pub(in crate::game) fn sc_cycle_tick(&mut self) {
        if self.station.as_ref().is_some_and(|st| {
            st.__horizon_mixins_strangely_charming_cycle_pending()
                .replace(false)
        }) {
            self.sc_request_cycle();
        }
    }
    fn sc_request_cycle(&mut self) {
        if !self.sc_cycle_ready() {
            return;
        }
        if self.remote.is_some() {
            let mut data = vec![3];
            data.extend_from_slice(&self.remote_menu_id().to_be_bytes());
            self.send(Serverbound::CustomPayload(CustomPayload {
                channel: crate::game::strangely_charming::ACTION_CHANNEL.into(),
                data,
            }));
        } else if let Some(mut m) = self.station.as_mut().and_then(|s| s.merchant.take()) {
            let ok = self.sc_cycle_offers(&mut m);
            if let Some(s) = self.station.as_mut() {
                s.merchant = Some(m);
            }
            if !ok {
                return;
            }
        }
        self.audio.play("minecraft:ui.button.click", None, 0.25, 1.);
    }
    pub(in crate::game) fn sc_cycle_click(&mut self, local: (f32, f32)) -> bool {
        if !hit(local) || !self.sc_cycle_ready() {
            return false;
        }
        self.sc_request_cycle();
        true
    }
    pub(in crate::game) fn sc_draw_cycle(&self, batch: &mut HudBatch) {
        if !self.sc_cycle_ready() {
            return;
        }
        let (ox, oy) = self.inventory_origin(batch);
        let (x, y, w, h) = BUTTON;
        let atlas = &self.gui.atlas;
        let sprite = if hit((self.cursor.0 - ox, self.cursor.1 - oy)) {
            "gui/sprites/widget/button_highlighted"
        } else {
            "gui/sprites/widget/button"
        };
        batch.sprite_scaled(atlas, sprite, ox + x, oy + y, w, h, u32::MAX);
        let title = "Cycle Trades";
        batch.text(
            atlas,
            title,
            ox + x + ((w - HudBatch::text_width(atlas, title)) / 2.).floor(),
            oy + y + 4.,
            u32::MAX,
            true,
        );
    }
}
fn reset(m: &mut MerchantState) {
    m.scroll = 0;
    m.focused = None;
    m.dragging = false;
    m.session.selection_hint = 0;
    m.session.active = None;
    m.session.future_xp = 0;
}

pub(in crate::game) fn sort_books(s: &mut horizon_client::merchant::Session) {
    if s.profession == "minecraft:librarian"
        && fresh(s)
        && horizon_mods::evanders_config::enabled("librarian_book_first")
    {
        horizon_client::merchant::strangely_charming_cycle::book_first(&mut s.offers);
    }
}
