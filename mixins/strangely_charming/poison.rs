use super::{Mob, Species, villager};
use horizon_world::nbt::{Compound, Tag};
pub const STATE: &str = "strangely_charming:poison_apple";
pub fn eligible(m: &Mob) -> bool {
    m.species() == Species::Villager
        && m.alive()
        && !m.removed
        && !m.is_baby()
        && m.extra.variant2 != villager::NITWIT
}
pub fn infect(z: &mut Mob, original_no_ai: bool) -> Tag {
    z.persistent = true;
    std::sync::Arc::make_mut(&mut z.kind).flags.burns_in_day = false;
    z.no_ai = true;
    z.velocity = horizon_core::geometry::Vec3::ZERO;
    z.add_effect_with("minecraft:weakness", 5 * 60 * 20, 99, false, true);
    z.add_effect("minecraft:slowness", 100, 99);
    let mut state = Compound::new();
    state.insert("stun".into(), Tag::Int(100));
    state.insert("original_no_ai".into(), Tag::Byte(i8::from(original_no_ai)));
    Tag::Compound(state)
}
pub fn before_tick(m: &mut Mob, raw: &mut Tag, ticking: bool) {
    if m.species() != Species::ZombieVillager || m.removed {
        return;
    }
    let Tag::Compound(raw) = raw else {
        return;
    };
    let Some(Tag::Compound(state)) = raw.get_mut(&STATE.into()) else {
        return;
    };
    if m.kind.flags.burns_in_day {
        std::sync::Arc::make_mut(&mut m.kind).flags.burns_in_day = false;
    }
    if !ticking {
        return;
    }
    let original = matches!(state.get(&"original_no_ai".into()), Some(Tag::Byte(v)) if *v != 0);
    if let Some(Tag::Int(ticks)) = state.get_mut(&"stun".into()) {
        m.no_ai = original || *ticks > 0;
        if *ticks > 0 {
            *ticks -= 1;
        } else {
            state.remove(&"stun".into());
            state.remove(&"original_no_ai".into());
        }
    }
}
