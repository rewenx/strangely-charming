use super::{
    StructureResources,
    start::{Piece, StartContext, StructureRegistry, StructureStart, try_generate},
};
use crate::{heightmap::HeightmapType, worldgen::block_feature::FeatureBlockBindings};
use horizon_core::position::ChunkPos;
pub(crate) const FREE: &str = "strangely_charming:rune_circle";
pub(crate) const CIRCLES: [(&str, &str); 3] = [
    (
        "minecraft:trial_chambers",
        "strangely_charming:rune_circle_trial",
    ),
    (
        "minecraft:stronghold",
        "strangely_charming:rune_circle_stronghold",
    ),
    (
        "minecraft:ancient_city",
        "strangely_charming:rune_circle_ancient_city",
    ),
];

use crate::worldgen::block_predicate::PredicateBlockResolver;

// Resolve biome names in this world's registry. Resource packs may change numeric IDs.
fn matches_biome(
    bindings: &FeatureBlockBindings<'_>,
    biome: crate::worldgen::biome::BiomeId,
    json: &str,
) -> bool {
    bindings
        .predicate_biome_set(json)
        .is_ok_and(|set| set.contains(&biome))
}

// Called after vanilla has evaluated placement, biome, competing entries and layout.
pub(crate) fn attach(
    registry: &StructureRegistry,
    resources: &StructureResources,
    bindings: &FeatureBlockBindings<'_>,
    context: &mut impl StartContext,
    chunk: ChunkPos,
    starts: &mut Vec<StructureStart>,
) {
    let anchored = CIRCLES
        .iter()
        .filter(|(parent, _)| starts.iter().any(|s| s.name == *parent))
        .copied()
        .collect::<Vec<_>>();
    if let Some(free) = starts.iter_mut().find(|s| s.name == FREE) {
        style_circle(free, registry, resources, bindings, context);
    }
    if anchored.is_empty() || !registry.sets.iter().any(|set| set.name == FREE) {
        return;
    }
    // Current-version saves do not run Horizon's old-terrain blending path. A
    // previously saved dungeon without a marker must not advertise a phantom ring.
    let saved = context.saved_starts(chunk);
    for (parent, marker) in anchored {
        if saved.iter().any(|s| s.name == parent) && !saved.iter().any(|s| s.name == marker) {
            continue;
        }
        let Some(i) = registry.structures.iter().position(|s| s.name == marker) else {
            continue;
        };
        let Some(mut circle) = try_generate(registry, resources, bindings, context, i, chunk, true)
        else {
            continue;
        };
        style_circle(&mut circle, registry, resources, bindings, context);
        starts.retain(|s| s.name != FREE);
        starts.push(circle);
    }
}

fn style_circle(
    circle: &mut StructureStart,
    registry: &StructureRegistry,
    resources: &StructureResources,
    bindings: &FeatureBlockBindings<'_>,
    context: &mut impl StartContext,
) {
    if let [Piece::Pool(piece)] = circle.pieces.as_mut_slice() {
        let bb = piece.bounding_box;
        let mid_x = (bb.min[0] + bb.max[0]) / 2;
        let mid_z = (bb.min[2] + bb.max[2]) / 2;
        let center_floor = context.base_height(mid_x, mid_z, HeightmapType::OceanFloorWg);
        let biome = context.noise_biome([mid_x >> 2, center_floor >> 2, mid_z >> 2]);
        let sand = matches_biome(bindings, biome, r#""minecraft:desert""#);
        let vines = matches_biome(
            bindings,
            biome,
            r#"["minecraft:jungle","minecraft:sparse_jungle","minecraft:bamboo_jungle","minecraft:swamp","minecraft:mangrove_swamp"]"#,
        );
        let grassy = matches_biome(
            bindings,
            biome,
            r#"["minecraft:plains","minecraft:sunflower_plains","minecraft:meadow","minecraft:savanna","minecraft:forest","minecraft:flower_forest","minecraft:birch_forest","minecraft:old_growth_birch_forest","minecraft:dark_forest","minecraft:cherry_grove"]"#,
        );
        let mut low = center_floor;
        let mut high = center_floor;
        let mut wet = false;
        // Sample the eight actual pillars plus the center, invariant under ring rotation.
        for (dx, dz) in [
            (0, 0),
            (0, 6),
            (0, -6),
            (6, 0),
            (-6, 0),
            (4, 4),
            (4, -4),
            (-4, 4),
            (-4, -4),
        ] {
            let (x, z) = (mid_x + dx, mid_z + dz);
            let ground = context.base_height(x, z, HeightmapType::OceanFloorWg);
            low = low.min(ground);
            high = high.max(ground);
            wet |= context.base_height(x, z, HeightmapType::WorldSurfaceWg) > ground;
        }
        let buried = wet || high - low > 2;
        let style = if sand {
            Some("sand")
        } else if vines {
            Some("overgrown")
        } else if wet {
            Some("seabed")
        } else if grassy {
            Some(if buried { "mossy_buried" } else { "mossy_ring" })
        } else if buried {
            Some("weathered")
        } else {
            None
        };
        if let Some(style) = style
            && let super::jigsaw::PoolElement::Single { template, .. } = piece.element.as_ref()
            && let Some(variant) = template
                .location
                .strip_prefix("strangely_charming:circle/ring_")
                .and_then(|v| v.parse::<usize>().ok())
            && let Some(pool) = registry.pools.get(
                &format!("strangely_charming:circle/{style}"),
                resources,
                bindings,
            )
            && let Some(element) = pool.templates.get(variant)
        {
            piece.element = element.clone();
            // Fix the archaeological layer against the low side. Never raise a pillar
            // to follow a hillside, or carve a cavity in the ground above its cap.
            piece.position[1] = if buried { low - 3 } else { low - 1 };
            // Center the wider feathered skirt around the original ring for all rotations.
            let bounds = piece.element.bounding_box(piece.position, piece.rotation);
            piece.position[0] += mid_x - (bounds.min[0] + bounds.max[0]).div_euclid(2);
            piece.position[2] += mid_z - (bounds.min[2] + bounds.max[2]).div_euclid(2);
            piece.bounding_box = piece.element.bounding_box(piece.position, piece.rotation);
            circle.bounding_box = piece.bounding_box;
        }
    }
}
