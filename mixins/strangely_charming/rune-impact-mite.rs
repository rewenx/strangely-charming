fn hook(kind: &str) -> core::ops::ControlFlow<Option<u32>> {
    if kind == "minecraft:endermite" && crate::game::sc_runes::RUNE_IMPACT.get() {
        core::ops::ControlFlow::Break(None)
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
