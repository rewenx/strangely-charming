fn hook(mods: &mut Vec<NativeMod>) {
    let keys = horizon_mods::evanders_config::KEYS.iter().copied()

    .filter(|(key, _, _)| horizon_mods::evanders_config::enabled(key))
    .map(|(id, label, key)| [id.into(), label.into(), key.into()])
    .collect();
    mods.push(NativeMod {
        id: "strangely_charming".into(),
        title: "Strangely Charming 0.2.42".into(),
        description: "Gameplay tweaks".into(),
        keys,
    });
}
