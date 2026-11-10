fn hook(projectiles: &mut [Projectile], id: u32, stack: &ItemStack) {
    if stack.item.as_ref() == crate::game::sc_runes::PEARL
        && let Some(p) = projectiles
            .iter_mut()
            .find(|p| p.id == id && matches!(p.kind, ProjectileKind::EnderPearl))
    {
        p.item = Some(stack.clone());
    }
}
