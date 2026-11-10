fn hook(damage_type: &str) -> core::ops::ControlFlow<()> {
    if damage_type == "minecraft:ender_pearl" && crate::game::sc_runes::RUNE_IMPACT.get() {
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
