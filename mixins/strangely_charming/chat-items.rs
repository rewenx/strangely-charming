use super::{Game, Screen, lan::LanHost};
use horizon_client::{
    item_data::encode_components, items::ItemStack, lan::data::component_nbt, text::Text,
};
use horizon_world::nbt::Tag;
use std::{
    cell::RefCell,
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    time::{Duration, Instant},
};
const CHANNEL: &str = "strangely_charming:share_item";
thread_local! {
 static HOVER:RefCell<Option<u64>>=const{RefCell::new(None)};
 static SEND:RefCell<Option<u64>>=const{RefCell::new(None)};
 static LIMIT:RefCell<HashMap<u128,Instant>>=RefCell::new(HashMap::new());
}
pub(super) fn signature(s: &ItemStack) -> u64 {
    let mut h = DefaultHasher::new();
    s.item.hash(&mut h);
    s.count.hash(&mut h);
    if let Some(c) = encode_components(s) {
        horizon_world::snbt_text::to_snbt(&c).hash(&mut h);
    }
    h.finish()
}
pub(super) fn hover(s: &ItemStack) {
    HOVER.with(|v| v.replace(Some(signature(s))));
}
impl Game<'_> {
    pub(super) fn sc_share_input(&self, name: &str, repeat: bool) -> bool {
        if repeat
            || !horizon_mods::evanders_config::enabled("item_sharing")
            || self.title.is_some()
            || !matches!(self.screen, Screen::Inventory | Screen::Creative)
            || self.settings.keys.get("key.chat") != name
            || !self.has_shift_down()
            || self.recipe_search_focused()
            || self.station_typing()
            || self.mod_browser_typing()
        {
            return false;
        }
        HOVER.with(|h| {
            if let Some(id) = *h.borrow() {
                SEND.with(|v| v.replace(Some(id)));
                true
            } else {
                false
            }
        })
    }
    pub(super) fn sc_share_frame(&mut self) {
        HOVER.with(|h| h.replace(None));
        let Some(id) = SEND.with(|v| v.take()) else {
            return;
        };
        if let Some(remote) = self.remote.as_mut() {
            remote
                .session
                .send_custom_click(CHANNEL, Some(Tag::Long(id as i64)));
        } else {
            let held = self
                .inventory
                .iter()
                .chain(&self.armor)
                .chain([&self.offhand])
                .flatten()
                .find(|s| signature(s) == id)
                .cloned();
            if let Some(s) = held {
                let line = self.sc_link_line(self.acting_player_name(), &s);
                self.system_message(line);
            }
        }
    }
    fn sc_link_line(&self, name: &str, s: &ItemStack) -> Text {
        let mut hover =
            serde_json::json!({"action":"show_item","id":s.item.as_ref(),"count":s.count});
        if let Some(c) = encode_components(s) {
            hover["components"] = horizon_client::scoreboard::tag_to_json(&c);
        }
        let label = self.stack_display_name(s);
        let mut c = label.component();
        c["hover_event"] = hover;
        Text::literal(format!("<{name}> ")).append(label.with_component(c))
    }
    pub(super) fn sc_share_packet(&mut self, host: &mut LanHost, conn: u32, data: Option<&Tag>) {
        if !horizon_mods::evanders_config::enabled("item_sharing") {
            return;
        }
        let Some(Tag::Long(id)) = data else { return };
        let Some(g) = host
            .hub
            .guest(conn)
            .filter(|g| g.placed && !g.dead && g.info.chat_visibility == 0)
        else {
            return;
        };
        if !LIMIT.with_borrow_mut(|m| {
            m.retain(|_, t| t.elapsed() < Duration::from_secs(2));
            m.insert(g.uuid, Instant::now()).is_none()
        }) {
            return;
        }
        // The request carries only a fingerprint. The server constructs the actual
        // hover payload from the inventory or visible slots it last sent this guest.
        let menu = g.menu.iter().flat_map(|m| {
            m.sent
                .iter()
                .enumerate()
                .filter(|(i, _)| !m.hidden.contains(i))
                .map(|(_, s)| s)
        });
        let Some(s) = g
            .inventory
            .iter()
            .chain(&g.armor)
            .chain([&g.offhand, &g.carried, &g.craft_result])
            .chain(g.craft_grid.iter())
            .chain(menu)
            .flatten()
            .find(|s| signature(s) == *id as u64)
            .or_else(|| {
                let play = host.play.get(&conn)?;
                let pred = |s: &ItemStack| signature(s) == *id as u64;
                // Use a borrowed slot for station, trade and mount windows; these
                // containers live outside Horizon's ordinary chest menu.
                play.station
                    .as_ref()
                    .and_then(|m| m.sc_find_link(&pred))
                    .or_else(|| play.merchant.as_ref().and_then(|m| m.sc_find_link(&pred)))
                    .or_else(|| {
                        play.mount_window
                            .as_ref()
                            .and_then(|m| m.sc_find_link(&pred))
                    })
                    .or_else(|| {
                        play.crafting.table.as_ref().and_then(|(_, _, m, _)| {
                            (0..m.slot_count()).find_map(|i| m.slot(i).filter(|s| pred(s)))
                        })
                    })
                    .or_else(|| {
                        play.crafting.machine.as_ref().and_then(|(_, m)| {
                            (0..m.slot_count()).find_map(|i| m.slot(i).filter(|s| pred(s)))
                        })
                    })
            })
            .or_else(|| {
                g.creative()
                    .then(|| {
                        self.creative_screen
                            .search_items()
                            .iter()
                            .find(|s| signature(s) == *id as u64)
                    })
                    .flatten()
            })
        else {
            return;
        };
        let line = self.sc_link_line(&g.name, s);
        let tag = component_nbt(&line.component());
        if horizon_world::snbt_text::to_snbt(&tag).len() < 32000 {
            let name = line.plain();
            if !self.chat_filter_text(
                host,
                conn,
                &[&name],
                horizon_client::lan::hub::chat_filter::Place::Chat,
            ) {
                return;
            }
            let packet = horizon_protocol::packets::play::Clientbound::SystemChat(
                horizon_protocol::packets::play::SystemChat {
                    content: tag,
                    overlay: false,
                },
            );
            for peer in host
                .hub
                .guests
                .iter()
                .filter(|g| g.placed && g.info.chat_visibility == 0)
            {
                peer.send(&packet);
            }
        }
    }
}
