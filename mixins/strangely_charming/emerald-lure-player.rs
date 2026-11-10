fn hook(
    mob: &Mob,
    host: Option<&PlayerView>,
    guests: &GuestViews,
) -> core::ops::ControlFlow<Option<u32>> {
    use core::ops::ControlFlow;
    match horizon_client::mobs::strangely_charming_emerald_lure::nearest_holder(mob, host, guests) {
        Some(connection) => ControlFlow::Break(connection),
        None => ControlFlow::Continue(()),
    }
}
