fn hook(id: &str, table: &mut Value) {
    if crate::evanders_config::enabled("rune_travel")
        && (matches!(
            id,
            "minecraft:chests/abandoned_mineshaft"
                | "minecraft:chests/ancient_city"
                | "minecraft:chests/ancient_city_ice_box"
                | "minecraft:chests/stronghold_corridor"
                | "minecraft:chests/stronghold_crossing"
                | "minecraft:chests/stronghold_library"
        ) || matches!(
            id.strip_prefix("minecraft:chests/trial_chambers/"),
            Some("corridor" | "entrance" | "intersection" | "intersection_barrel" | "supply")
        ))
        && let Some(pools) = table["pools"].as_array_mut()
    {
        pools.push(serde_json::json!({"rolls":1,"condition":{"type":"minecraft:random_chance","chance":0.20},"entries":[{"type":"minecraft:item","name":"strangely_charming:rune_dust","modifier":[{"type":"minecraft:set_count","count":{"type":"minecraft:uniform","min":2,"max":4}}]}]}));
    }
    if matches!(id, "strangely_charming:chests/underground_ruins" | "strangely_charming:chests/nether_halls")
        && let Some(pools) = table["pools"].as_array_mut()
    {
        let bonus = |name: &str, chance: f32, low: i32, high: i32| serde_json::json!({
            "rolls":1, "condition":{"type":"minecraft:random_chance","chance":chance},
            "entries":[{"type":"minecraft:item","name":name,"modifier":[{"type":"minecraft:set_count",
                "count":{"type":"minecraft:uniform","min":low,"max":high}}]}]
        });
        if crate::evanders_config::enabled("rune_travel") {
            pools.push(bonus("strangely_charming:rune_dust", 0.20, 3, 6));
            pools.push(bonus("strangely_charming:rune_pearl", 0.10, 1, 1));
        }
        if crate::evanders_config::enabled("ender_pearl_blocks") {
            pools.push(bonus("strangely_charming:ender_pearl_block", 0.08, 1, 1));
        }
    }
    let bonus: Option<(&str, f32, &[&str])> = match id {
        "minecraft:chests/end_city_treasure" => {
            Some(("enchanting_totems", 0.125, &["totem_of_enchanting"]))
        }
        "minecraft:chests/simple_dungeon" | "minecraft:chests/village/village_cartographer" => {
            Some((
                "scrollkeepers",
                0.20,
                &[
                    "scroll_wheat",
                    "scroll_bread",
                    "scroll_coal",
                    "scroll_zombies",
                ],
            ))
        }
        _ => None,
    };
    if let Some((flag, chance, items)) = bonus
        && crate::evanders_config::enabled(flag)
        && let Some(pools) = table["pools"].as_array_mut()
    {
        let entries: Vec<_>=items.iter().map(|item|serde_json::json!({"type":"minecraft:item","name":format!("strangely_charming:{item}")})).collect();
        pools.push(serde_json::json!({"rolls":1,"condition":{"type":"minecraft:random_chance","chance":chance},"entries":entries}));
    }
    let (key, items): (&str, &[(&str, i32)]) = match id {
        "minecraft:chests/woodland_mansion" => (
            "mansion_loot",
            &[
                ("totem_of_undying", 1),
                ("diamond_axe", 3),
                ("diamond_chestplate", 2),
                ("enchanted_book", 4),
            ],
        ),
        "minecraft:chests/nether_bridge" => (
            "fortress_loot",
            &[
                ("wither_skeleton_skull", 1),
                ("diamond", 5),
                ("blaze_powder", 5),
                ("golden_sword", 3),
                ("enchanted_book", 3),
            ],
        ),
        _ => return,
    };
    if !crate::evanders_config::enabled(key) {
        return;
    }
    if let Some(pools) = table["pools"].as_array_mut() {
        let entries: Vec<_> = items.iter().map(|&(item,weight)| {
            let mut entry = serde_json::json!({"type":"minecraft:item","name":format!("minecraft:{item}"),"weight":weight});
            if item=="enchanted_book" {
                entry["name"]=serde_json::json!("minecraft:book");
                entry["modifier"]=serde_json::json!([{"type":"minecraft:enchant_randomly","options":"#minecraft:treasure"}]);
            } else if matches!(item,"diamond_axe" | "diamond_chestplate" | "golden_sword") {
                entry["modifier"] = serde_json::json!([{"type":"minecraft:enchant_randomly"}]);
            }
            entry
        }).collect();
        pools.push(serde_json::json!({"rolls":1,"entries":entries}));
    }
}
