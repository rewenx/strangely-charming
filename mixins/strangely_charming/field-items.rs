use super::Game;
use horizon_core::random::{RandomSource, slime_chunk_random};
use horizon_world::nbt::Tag;

pub(super) const KNOWLEDGE: &str = "strangely_charming:knowledge_stone";
pub(super) const SLIME: &str = "strangely_charming:slime_stone";

impl Game<'_> {
    pub(super) fn sc_private_chat(&mut self, message: &str) {
        if self.lan_acting.is_some() {
            use horizon_protocol::packets::play::{Clientbound, SystemChat};
            self.guest_packets.push(Clientbound::SystemChat(SystemChat {
                content: Tag::String(message.into()),
                overlay: false,
            }));
        } else {
            self.system_message(horizon_client::text::Text::literal(message));
        }
    }
    pub(super) fn sc_use_slime(&mut self) {
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let at = [p.position.x, p.position.y, p.position.z];
        if self.dim.id() == "minecraft:overworld"
            && slime_chunk_random(
                (at[0].floor() as i32) >> 4,
                (at[2].floor() as i32) >> 4,
                self.level.seed,
                987_234_911,
            )
            .next_int_bound(10)
                == Ok(0)
        {
            let who = self.native_acting_id();
            self.native_play_sound_to(
                who,
                "minecraft:entity.slime.jump",
                "player",
                Some(at),
                1.0,
                1.0,
            );
            self.sc_private_chat("this area feels slimy...");
        }
    }
    pub(super) fn sc_unlock_recipes(&mut self) -> bool {
        let previously_granted = self.sc_research().ok().is_some_and(|r| r.all_recipes);
        let index = &world_stations!(self).recipe_index;
        let ids = index
            .ids()
            .filter(|id| !index.special(id))
            .cloned()
            .collect::<Vec<_>>();
        if previously_granted && ids.iter().all(|id| self.level.recipe_book.knows(id)) {
            return false;
        }
        let (_, unlocked) = self.level.recipe_book.add_recipes(index, &ids);
        self.sc_flag_recipes();
        if unlocked.is_empty() {
            return false;
        }
        let quiet = std::mem::replace(&mut self.adv.quiet, true);
        let prior = self.adv.unlocked.len();
        self.recipe_unlocked(&unlocked);
        self.adv.quiet = quiet;
        if self.lan_acting.is_some() {
            if self.adv.unlocked.len() > prior {
                // Request one notification-free full book packet in this player's next flush.
                self.adv.unlocked.truncate(prior);
                if !self.adv.unlocked.iter().any(String::is_empty) {
                    self.adv.unlocked.push(String::new());
                }
            }
            use horizon_protocol::packets::play::{Clientbound, SystemChat};
            self.guest_packets.push(Clientbound::SystemChat(SystemChat {
                content: Tag::String("all recipes unlocked".into()),
                overlay: false,
            }));
        } else {
            self.system_message(horizon_client::text::Text::literal("all recipes unlocked"));
        }
        true
    }
}
