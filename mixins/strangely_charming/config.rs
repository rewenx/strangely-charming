use std::{collections::BTreeMap, fs, io, path::Path, sync::OnceLock};
const FEATURES: &[(&str, &str)] = &[
    ("item_sharing", "Link a hovered inventory item in chat with Shift and your chat key."),
    ("grave_wayfinder", "Show the direction to your latest unclaimed grave within 100 blocks."),
    ("gravestones", "Keep death inventories in owner-only gravestones."),
    (
        "beekeepers",
        "Let villagers claim beehives and trade bee supplies.",
    ),
    (
        "coral_squids",
        "Spawn coral squids near living coral in warm oceans.",
    ),
    (
        "crying_obsidian_frames",
        "Allow crying obsidian in Nether portal frames.",
    ),
    (
        "cycle_villager_trades",
        "Reroll an untraded villager’s offers from the trade window.",
    ),
    (
        "enchanting_totems",
        "Add reusable totems that store and return XP.",
    ),
    (
        "ender_pearl_blocks",
        "Guide nearby chorus fruit teleports to Ender Pearl Blocks.",
    ),
    (
        "exploration_advancements",
        "Add advancements for discovering the mod’s features.",
    ),
    (
        "extra_recipes",
        "Add crafting recipes for supplies and salvage.",
    ),
    (
        "extract_enchantments",
        "Transfer enchantments onto books at a grindstone.",
    ),
    ("fortress_loot", "Add treasure to Nether fortress chests."),
    ("fullbright", "Toggle full brightness with a hotkey."),
    (
        "grindable_armor",
        "Recover materials from undamaged, unenchanted armor at a grindstone.",
    ),
    (
        "inventory_tidying",
        "Merge stacks and sort a backpack or open storage container.",
    ),
    (
        "inventory_totem",
        "Use a Totem of Undying from anywhere in your inventory.",
    ),
    (
        "kilns",
        "Add kilns that quickly fire clay, sand, and similar materials.",
    ),
    ("decorative_amethyst", "Craft and stonecut eight decorative amethyst blocks."),
    ("rune_circle_repair", "Restore damaged natural rune circles with a Restoration Ruby."),
    ("protogen_mishaps", "Summon hostile protogens during the two creature mishaps at unknown runes."),
    ("rune_travel", "Travel through runes, study glyphs, and build journal portals."),
    ("knowledge_stone", "Learn missing recipes or light rune portals with a Knowledge Stone."),
    (
        "librarian_book_first",
        "List an enchanted book first when a novice librarian rolls one.",
    ),
    (
        "lumberjacks",
        "Add Table Saws for woodcutting and lumberjack villagers.",
    ),
    ("mansion_loot", "Add treasure to woodland mansion chests."),
    (
        "mineshaft_ores",
        "Generate ore in stone and deepslate mineshaft floors.",
    ),
    (
        "moonstones",
        "Generate glowing moonstone geodes in the outer End.",
    ),
    (
        "poison_apple",
        "Turn an eligible adult villager into a daylight-safe zombie villager.",
    ),
    (
        "portable_crafting",
        "Open a crafting grid while carrying a crafting table.",
    ),
    (
        "protect_pets",
        "Prevent accidental attacks on your own pets.",
    ),
    (
        "raid_horns",
        "Let raid captains drop a reusable horn that ends nearby raids.",
    ),
    (
        "rune_circles",
        "Generate rune circles in grasslands and above three Overworld structures.",
    ),
    (
        "runebreaker",
        "Add the Runebreaker tool for destroying carved runestones.",
    ),
    (
        "scrollkeepers",
        "Add Writing Desks and Scrollkeepers who offer local quests.",
    ),
    ("slime_stone", "Use a Slime Stone to test for an Overworld slime chunk."),
    ("spade", "Place a random block from your hotbar with a wooden Spade."),
    (
        "stackable_enchanted_books",
        "Stack matching enchanted books up to 16.",
    ),
    ("stackable_potions", "Stack matching potions up to 16."),
    (
        "stackable_stews",
        "Stack matching mushroom stew, rabbit stew, or beetroot soup up to 16.",
    ),
    (
        "travel_journal",
        "Record locations and photos in a journal, share pages, and make marked maps.",
    ),
    (
        "underground_ruins",
        "Generate buried ruins in snowy Overworld biomes and the Nether.",
    ),
    (
        "unlimited_anvil",
        "Allow survival anvil work costing more than 39 levels.",
    ),
    (
        "villagers_follow_emerald_blocks",
        "Let nearby villagers follow a player holding an emerald block.",
    ),
];
const FREQUENCY: &str = "rune_circle_frequency";
const FREQUENCY_NOTE: &str = "Unanchored rune circle frequency in percent (100 = normal); only affects newly generated chunks.";
type Settings = BTreeMap<String, serde_json::Value>;
static CONFIG: OnceLock<Settings> = OnceLock::new();

// Replace JSONC comments with whitespace, respecting quoted keys and preserving
// line numbers for serde's diagnostics. Feature switches remain booleans.
fn parse_config(source: &str) -> io::Result<Settings> {
    let mut clean = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let (mut string, mut escaped) = (false, false);
    while let Some(c) = chars.next() {
        if string {
            clean.push(c);
            if escaped {
                escaped = false;
            } else if c == char::from(92) {
                escaped = true;
            } else if c == '"' {
                string = false;
            }
        } else if c == '/' && chars.next_if_eq(&'/').is_some() {
            clean.push(' ');
            for c in chars.by_ref() {
                if c == '\n' || c == '\r' {
                    clean.push(c);
                    break;
                }
            }
        } else if c == '/' && chars.next_if_eq(&'*').is_some() {
            clean.push(' ');
            loop {
                match chars.next() {
                    Some('*') if chars.next_if_eq(&'/').is_some() => break,
                    Some(c @ ('\n' | '\r')) => clean.push(c),
                    Some(_) => (),
                    None => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "unclosed config comment",
                        ));
                    }
                }
            }
        } else {
            clean.push(c);
            string = c == '"';
        }
    }
    serde_json::from_str(&clean).map_err(io::Error::other)
}

fn render(values: &Settings) -> io::Result<String> {
    let mut lines = Vec::new();
    for (key, value) in values {
        let comment = FEATURES
            .iter()
            .find(|(id, _)| id == key)
            .map(|(_, note)| format!("  // {note}\n"))
            .unwrap_or_else(|| if key == FREQUENCY { format!("  // {FREQUENCY_NOTE}\n") } else { String::new() });
        lines.push(format!(
            "{comment}  {}: {value}",
            serde_json::to_string(key)?
        ));
    }
    Ok(format!("{{\n{}\n}}\n", lines.join(",\n")))
}

fn load() -> io::Result<Settings> {
    let path = Path::new("config/strangely_charming.jsonc");
    let mut values = BTreeMap::new();
    let mut changed = true;
    for source in [
        path,
        Path::new("config/strangely_charming.json"),
        Path::new("config/evanders_tweaks.json"),
    ] {
        match fs::read_to_string(source) {
            Ok(text) => {
                values = parse_config(&text)?;
                changed = source != path;
                break;
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
    }
    changed |= values.remove("bundle_recipe").is_some();
    changed |= values.remove("mooblooms").is_some();
    if !values.get(FREQUENCY).is_some_and(|v| v.as_u64().is_some()) {
        values.insert(FREQUENCY.into(), 100u64.into());
        changed = true;
    }
    for &(key, _) in FEATURES {
        if !values.get(key).is_some_and(|v| v.is_boolean()) {
            values.insert(key.to_owned(), true.into());
            changed = true;
        }
    }
    if changed {
        fs::create_dir_all(path.parent().unwrap())?;
        let temp = path.with_extension(format!(
            "{}-{}.tmp",
            std::process::id(),
            env!("CARGO_PKG_NAME")
        ));
        fs::write(&temp, render(&values)?)?;
        fs::rename(temp, path)?;
    }
    Ok(values)
}
fn settings() -> &'static Settings {
    CONFIG.get_or_init(|| load().unwrap_or_else(|err| {
        eprintln!("Strangely Charming could not read its config: {err}. Using defaults for this session.");
        BTreeMap::new()
    }))
}

pub fn enabled(key: &str) -> bool {
    settings().get(key).and_then(serde_json::Value::as_bool).unwrap_or(true)
}
pub fn rune_circle_frequency() -> u64 {
    settings().get(FREQUENCY).and_then(serde_json::Value::as_u64).unwrap_or(100)
}

pub const KEYS:&[(&str,&str,&str)]=&[
        ("travel_journal","Look Up Journal Entry (hold)","key.keyboard.left.control"),
        ("cycle_villager_trades","Cycle Trades","key.keyboard.c"),
        ("scrollkeepers","Open Quest Journal","key.keyboard.j"),
        ("fullbright", "Toggle Fullbright", "key.keyboard.g"),
        ("portable_crafting", "Portable Crafting", "key.keyboard.v"),
        ("inventory_tidying", "Tidy Backpack", "key.mouse.middle"),
    ];
