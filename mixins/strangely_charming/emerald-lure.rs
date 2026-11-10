use super::{
    Mob, PlayerView, Species,
    goals::{Cx, Goal, Tempt},
    villager,
};

const BLOCK: &str = "minecraft:emerald_block";

#[derive(Clone, Debug)]
struct EmeraldBlockTempt(Tempt);

pub fn register(mob: &mut Mob) {
    if mob.species() == Species::Villager
        && horizon_mods::evanders_config::enabled("villagers_follow_emerald_blocks")
        && !mob.ai.goals.has("Tempt")
        && !mob.ai.goals.has("EmeraldBlockTempt")
    {
        mob.ai.goals.add(
            2,
            EmeraldBlockTempt(Tempt::new(0.6, [BLOCK.to_owned()].into(), false)),
        );
    }
}

fn free_to_follow(mob: &Mob) -> bool {
    // A villager with an open trading session must keep its customer.
    !mob.extra.flag2 && !villager::sleeping(mob)
}

impl Goal for EmeraldBlockTempt {
    fn flags(&self) -> u8 {
        self.0.flags()
    }

    fn can_use(&mut self, mob: &mut Mob, cx: &mut Cx) -> bool {
        free_to_follow(mob)
            && cx.w.player.as_ref().is_some_and(|p| !p.spectator)
            && self.0.can_use(mob, cx)
    }

    fn start(&mut self, mob: &mut Mob, cx: &mut Cx) {
        self.0.start(mob, cx);
    }

    fn tick(&mut self, mob: &mut Mob, cx: &mut Cx) {
        self.0.tick(mob, cx);
    }

    fn stop(&mut self, mob: &mut Mob, cx: &mut Cx) {
        self.0.stop(mob, cx);
    }

    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// Horizon gives each mob one player view. Prefer the closest eligible holder
/// when there are guests. Some(None) selects the host; None lets Horizon choose.
pub fn nearest_holder(
    mob: &Mob,
    host: Option<&PlayerView>,
    guests: &[(u32, PlayerView)],
) -> Option<Option<u32>> {
    if mob.species() != Species::Villager
        || !mob.ai.goals.has("EmeraldBlockTempt")
        || !free_to_follow(mob)
        || mob.extra.leash.is_some()
        || mob.ai.last_hurt_by == Some(super::Who::Player)
            && mob.age as i32 - mob.ai.last_hurt_by_time <= 100
    {
        return None;
    }
    host.into_iter()
        .map(|p| (None, p))
        .chain(guests.iter().map(|(conn, p)| (Some(*conn), p)))
        .filter(|(_, p)| {
            p.alive
                && !p.spectator
                && (p.main_hand.as_deref() == Some(BLOCK) || p.off_hand.as_deref() == Some(BLOCK))
        })
        .map(|(conn, p)| (conn, mob.distance_sqr(p.position)))
        .filter(|(_, distance)| *distance <= 100.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(conn, _)| conn)
}
