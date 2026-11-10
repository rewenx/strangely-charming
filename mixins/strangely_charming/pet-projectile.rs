fn hook(
    game: &mut crate::game::Game<'_>,
    p: &Projectile,
    hit: Hit,
) -> core::ops::ControlFlow<Result<()>> {
    if matches!(p.kind, ProjectileKind::EnderPearl)
        && p.owner == Some(Who::Player)
        && p.guest == game.lan_acting
        && p.owner_uuid
            .is_none_or(|id| id == game.acting_player_uuid())
        && let Hit::Block { pos, .. } = hit
        && game.sc_rune_impact(
            pos,
            p.item
                .as_ref()
                .is_some_and(|s| s.item.as_ref() == crate::game::sc_runes::PEARL),
        )
    {
        return core::ops::ControlFlow::Break(Ok(()));
    }
    if let Hit::Entity {
        who: Who::Mob(id), ..
    } = hit
        && p.owner == Some(Who::Player)
        && horizon_mods::evanders_config::enabled("protect_pets")
    {
        let owner = p.owner_uuid.unwrap_or_else(|| game.acting_player_uuid());
        if game
            .mobs
            .iter()
            .any(|m| m.id == id && m.extra.tame && m.extra.owner == Some(owner))
        {
            return core::ops::ControlFlow::Break(Ok(()));
        }
    }
    if crate::game::sc_runes::is_rune_pearl(p) && !crate::game::sc_runes::RUNE_IMPACT.replace(true)
    {
        let _reset = crate::game::sc_runes::RuneImpact;
        return core::ops::ControlFlow::Break(game.projectile_hit(p, hit));
    }
    core::ops::ControlFlow::Continue(())
}
