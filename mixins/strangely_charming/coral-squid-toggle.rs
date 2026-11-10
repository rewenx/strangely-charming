fn hook(kind: &str) -> core::ops::ControlFlow<Option<u32>> {
    if kind == "strangely_charming:coral_squid"
        && !horizon_mods::evanders_config::enabled("coral_squids")
    {
        return core::ops::ControlFlow::Break(None);
    }
    core::ops::ControlFlow::Continue(())
}
