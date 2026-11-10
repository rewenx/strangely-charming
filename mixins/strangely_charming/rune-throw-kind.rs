fn hook(value: &str) -> &str {
    if value == crate::game::sc_runes::PEARL && crate::game::sc_runes::enabled() {
        "minecraft:ender_pearl"
    } else {
        value
    }
}
