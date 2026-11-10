fn hook(m: &Mob, w: &MobWorld, state: horizon_world::registry::BlockStateId) -> core::ops::ControlFlow<Option<(&'static str, f32, f32)>> {
    if m.kind.id == "strangely_charming:protogen" {
        // Ocelots use the sound of the block underfoot.
        let floor = &w.facts.get(state).sounds;
        return core::ops::ControlFlow::Break(Some((intern_sound(&floor.step), floor.volume * 0.15, floor.pitch)));
    }
    core::ops::ControlFlow::Continue(())
}
