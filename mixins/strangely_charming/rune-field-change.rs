fn hook(level: &crate::game::GameLevel<'_>, pos: BlockPos, old: BlockStateId) {
    let name = &level.facts.get(old).name;
    if name.starts_with("strangely_charming:runic_amethyst_")
        || name.starts_with("strangely_charming:rune_portal_")
    {
        let dim = match level.dimension {
            LevelDimension::Overworld => "minecraft:overworld",
            LevelDimension::Nether => "minecraft:the_nether",
            LevelDimension::End => "minecraft:the_end",
        };
        crate::game::sc_runes::frame_change(level.seed, dim, [pos.x, pos.y, pos.z]);
    }
}
