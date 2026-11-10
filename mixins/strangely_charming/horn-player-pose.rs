fn hook(m: &Mob, s: &State, p: &mut Poses, a: &Animations) -> core::ops::ControlFlow<bool> {
    if m.kind.id == "strangely_charming:protogen" {
        super::evanders_models::pose_protogen(s, p);
        return core::ops::ControlFlow::Break(true);
    }
    if m.extra.using_item <= 0 || !matches!(m.kind.anim, super::Anim::Humanoid { .. }) {
        return core::ops::ControlFlow::Continue(());
    }
    let held = if m.extra.using_off_hand {
        &m.extra.off_hand
    } else {
        &m.extra.main_hand
    };
    if held.as_deref() != Some("strangely_charming:raid_horn") {
        return core::ops::ControlFlow::Continue(());
    }
    // Substitute the horn only in the pose calculation; equipment rendering keeps the raid horn model.
    let mut avatar = m.clone();
    let used = if avatar.extra.using_off_hand {
        &mut avatar.extra.off_hand
    } else {
        &mut avatar.extra.main_hand
    };
    *used = Some("minecraft:goat_horn".into());
    super::anim::setup(&avatar, s, p, a);
    core::ops::ControlFlow::Break(true)
}
