fn hook(engine: &mut ParticleEngine, ctx: &AnimateContext<'_>, state: BlockStateId, pos: [i32; 3]) {
    let Some(id) = ctx.facts.get(state).name.strip_prefix("strangely_charming:rune_portal_") else { return };
    let (pattern, axis) = id.rsplit_once('_').unwrap_or(("", id));
    let (painting, tile) = pattern.rsplit_once('_')
        .and_then(|(name, n)| n.parse::<usize>().ok().filter(|t| *t < 9).map(|t| (name, t)))
        .unwrap_or((pattern, 4));
    let normal = if axis == "x" { 2 } else { 0 };
    let along = 2 - normal;
    // Colors refer to the artwork, not the biome's grass tint. The same neutral
    // sprites supply both faces of every portal. Keep the vanilla animation budget.
    let rgb = |n: u32| [(n >> 16) as f32 / 255., ((n >> 8) & 255) as f32 / 255., (n & 255) as f32 / 255.];
    let (first, second) = match painting {
        "plains" | "oak_forest" | "dappled_forest" => (0x87d45c, 0xd8e688),
        "jungle" | "lush_caves" => (0x38ce76, 0xa7ef5c),
        "birch_forest" => (0xcae679, 0x7cba49),
        "flower_forest" | "flower_meadow" => (0xeeb1d8, 0x96d778),
        "cherry_grove" => (0xffa6cd, 0xffe4f5),
        "dark_forest" | "swamp" | "mangrove_swamp" => (0x529270, 0xb4b958),
        "taiga" => (0x439986, 0xa5d9af),
        "pale_garden" | "mountains" | "stony_highlands" => (0xbac9cc, 0xecdfc9),
        "snowy_taiga" | "snowy_plains" | "ice_spikes" | "frozen_ocean" => (0x70c7ff, 0xe6f9ff),
        "desert" | "savanna" => (0xe3b567, 0xffdc97),
        "badlands" | "dripstone_caves" => (0xd87950, 0xe7b782),
        "sulfur_caves" => (0xdcc853, 0xf0ec8f),
        "ocean" | "warm_ocean" => (0x389dcf, 0x79ead9),
        "mushroom_fields" => (0xde7188, 0xc8a8ec),
        "deep_dark" => (0x2b8c99, 0x7edfd0),
        "soul_sand_valley" | "warped_forest" => (0x39e4db, 0xa0ffff),
        "basalt_deltas" => (0xf86232, 0xffc56a),
        "nether" | "nether_wastes" | "crimson_forest" => (0xf13823, 0xffb43e),
        "end_islands" | "end_center" => (0xbc81ff, 0xeae1ff),
        "unknown" => (0xfd5fca, 0x76f5e8),
        _ => (0x99b5da, 0xe2d8ff),
    };
    let nether = matches!(painting, "nether" | "nether_wastes" | "crimson_forest" | "warped_forest" | "soul_sand_valley" | "basalt_deltas");
    let void = matches!(painting, "end_islands" | "end_center" | "unknown");
    for sign in [-1., 1.] {
        let r = engine.random();
        let mut p = pos.map(|v| f64::from(v) + 0.5);
        p[along] += r.next_double() - 0.5;
        p[1] += r.next_double() - 0.5;
        p[normal] += sign * (0.12 + r.next_double() * 0.22);
        let flicker = r.next_double();
        let wind = r.next_double() - 0.5;
        let color = rgb(if flicker < 0.68 { first } else { second });
        let rgba = [color[0], color[1], color[2], 1.];
        if void {
            // Pull sparks out of the air and into the center of the whole aperture.
            let mut target = pos.map(|v| f64::from(v) + 0.5);
            target[along] += 1. - (tile % 3) as f64 + 0.35 * wind;
            target[1] += (tile / 3) as f64 - 1. + 0.2 * wind;
            p[normal] += sign * (0.5 + flicker);
            let mut mote = ParticleOptions::simple(T::Trail);
            mote.color = rgba;
            mote.trail = Some((target, 12 + (flicker * 10.) as i32));
            engine.add(&mote, p, [0.; 3]);
            if flicker > 0.82 { add(engine, T::EndRod, target, [0., 0.008, 0.]); }
        } else if nether {
            // A hot core climbs a short distance before dying away; the outer
            // ash drifts sideways. Warped and soul valleys burn blue.
            let mut top = p;
            top[1] += 0.65 + flicker * 0.65;
            top[along] += wind * 0.55;
            top[normal] += sign * 0.2;
            let mut spark = ParticleOptions::simple(T::Trail);
            spark.color = rgba;
            spark.trail = Some((top, 13 + (flicker * 8.) as i32));
            engine.add(&spark, p, [0.; 3]);
            if flicker > 0.68 {
                let flame = if matches!(painting, "soul_sand_valley" | "warped_forest") { T::SoulFireFlame } else { T::SmallFlame };
                add(engine, flame, p, [0., 0.045, 0.]);
            } else if flicker < 0.12 { add(engine, T::Smoke, top, [0., 0.02, 0.]); }
        } else {
            let mut v = [0., 0.10 + flicker * 0.20, 0.];
            v[along] = wind * 0.55;
            v[normal] = sign * 0.6;
            let mut mote = ParticleOptions::simple(T::DustColorTransition);
            mote.scale = 0.82;
            mote.color = rgba;
            mote.to_color = rgb(second);
            engine.add(&mote, p, v);
        }
    }
}
