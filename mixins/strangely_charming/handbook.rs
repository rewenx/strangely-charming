// A page and its contextual item links share one source of truth. The first item
// normally illustrates the page; illustrative overrides do not change contextual lookup.
use horizon_mods::evanders_config::KEYS;
const STACKABLES: &[(&str, &str)] = &[
    ("stackable_potions", "potions"),
    ("stackable_stews", "stews"),
    ("stackable_enchanted_books", "enchanted books"),
];
pub(super) fn binding_label(name: &str) -> String {
    match name {
        "" | "key.keyboard.unknown" => "[no hotkey set]".to_owned(),
        "key.keyboard.left.control" => "L-Ctrl".to_owned(),
        _ => horizon_client::keybinds::display_name(name).plain(),
    }
}
pub(super) struct Page {
    pub key: &'static str,
    pub title: &'static str,
    pub body: &'static str,
    pub items: &'static [&'static str],
}
impl Page {
    pub(super) fn enabled(&self) -> bool {
        if self.key == "stackables" {
            STACKABLES
                .iter()
                .any(|&(toggle, _)| horizon_mods::evanders_config::enabled(toggle))
        } else {
            horizon_mods::evanders_config::enabled(self.key)
        }
    }
    pub(super) fn description(&self, game: &super::Game<'_>) -> String {
        let mut body = self.body.to_owned();
        for &(id, _, default) in KEYS {
            let token = format!("{{{id}}}");
            if body.contains(&token) {
                let binding = game.native_mod_key("strangely_charming", id, default);
                body = body.replace(&token, &binding_label(&binding));
            }
        }
        if body.contains("{share_item}") {
            let key = binding_label(game.settings.keys.get("key.chat"));
            let chord = if key == "[no hotkey set]" {
                key
            } else {
                format!("Shift + {key}")
            };
            body = body.replace("{share_item}", &chord);
        }
        if body.contains("{stackables}") {
            let names: Vec<_> = STACKABLES
                .iter()
                .filter(|(key, _)| horizon_mods::evanders_config::enabled(key))
                .map(|(_, name)| *name)
                .collect();
            let list = match names.as_slice() {
                [a, b, c] => format!("{a}, {b}, and {c}"),
                [a, b] => format!("{a} and {b}"),
                [a] => a.to_string(),
                _ => String::new(),
            };
            body = body.replace("{stackables}", &list);
        }
        body
    }
    pub(super) fn illustration(&self) -> Option<&'static str> {
        Some(match self.key {
            "cycle_villager_trades" => "minecraft:enchanted_book",
            _ => return self.items.first().copied(),
        })
    }
}
macro_rules! pages {
    ($(($key:literal, $title:literal, $body:literal, [$($item:literal),*])),+ $(,)?) => {
        pub(super) const PAGES: &[Page] = &[$(Page { key: $key, title: $title, body: $body, items: &[$($item),*] }),+];
    };
}
pages![
    (
        "travel_journal",
        "Gazetteer",
        "Registering locations allows you find them more easily in the future. Encode the location to  a compass, map, or sharable paper. Eventually, you may even be able to build a portal...",
        ["strangely_charming:travel_journal"]
    ),
    (
        "travel_journal",
        "Location Sharing",
        "Location paper allows you to share locations. To make one, have paper in your inventory and view a location in the journal.",
        ["strangely_charming:travel_journal_page"]
    ),
    (
        "rune_travel",
        "Rune Travel",
        "Successful teleportation from a rune with a pearl always send you to the same location. This deterministic behavior makes it possible to set up travel hubs.",
        [
            "strangely_charming:rune_pearl",
            "strangely_charming:rune_dust",
            "strangely_charming:rune_dust_block"
        ]
    ),
    (
        "rune_travel",
        "Rune Portals",
        "Once you learn enough glyphs, you can construct your own portals from amethyst. Use the same layout shown in the gazetteer. Use a Knowledge Stone on the bottom center from either side to activate.",
        [
            "strangely_charming:runic_amethyst_0",
            "strangely_charming:portal_diagram"
        ]
    ),
    (
        "spade",
        "Spade",
        "This tool selects a block randomly from your hotbar to place. To increase the frequency, add more stacks of the same block to your bar. You cam build gradients of block patterns.",
        ["strangely_charming:spade"]
    ),
    (
        "item_sharing",
        "Link Items in Chat",
        "Hover over an inventory item and press {share_item} to link it in chat. Other players can hover over the link to inspect the item.",
        []
    ),
    (
        "gravestones",
        "Gravestones",
        "Upon death, your items are placed in a grave stone for safe keeping. Others cannot loot it for two hours. It also hides you on approach to avoid death loops.",
        [
            "strangely_charming:short_tombstone",
            "strangely_charming:ornate_tombstone",
            "strangely_charming:tall_tombstone"
        ]
    ),
    (
        "slime_stone",
        "Slime Stone",
        "This stone is always damp. It gets soaking wet when used in a slime chunk, thus it is useful for finding them.",
        ["strangely_charming:slime_stone"]
    ),
    (
        "knowledge_stone",
        "Knowledge Stone",
        "Use of a knowledge stone grants access to all the game's recipes. You can also use it to ignite runic portals.",
        ["strangely_charming:knowledge_stone"]
    ),
    (
        "portable_crafting",
        "Portable Crafting",
        "You don't need to place down your crafting table. Press {portable_crafting} while you hold one to open a full crafting grid.",
        ["minecraft:crafting_table"]
    ),
    (
        "inventory_tidying",
        "Inventory Sorting",
        "Press {inventory_tidying} to sort your inventory or the container you have open.",
        []
    ),
    (
        "extra_recipes",
        "More Recipes",
        "Clay, snow, and quartz blocks may be crafted back into pieces, and you no longer need a full crafting grid to make paper from suger cane!",
        []
    ),
    (
        "stackables",
        "Stackable Items",
        "{stackables} with the same attributes may now stack up to sixteen!",
        [
            "minecraft:potion",
            "minecraft:splash_potion",
            "minecraft:lingering_potion",
            "minecraft:mushroom_stew",
            "minecraft:rabbit_stew",
            "minecraft:beetroot_soup",
            "minecraft:suspicious_stew",
            "minecraft:enchanted_book"
        ]
    ),
    (
        "unlimited_anvil",
        "Unlimited Anvil",
        "Anvils no longer have an XP cap or read \"too expensive.\"",
        [
            "minecraft:anvil",
            "minecraft:chipped_anvil",
            "minecraft:damaged_anvil"
        ]
    ),
    (
        "extract_enchantments",
        "Extract Enchantments",
        "Adding a plain book in a grindstone with an enchanted item let you keep its enchantments!",
        ["minecraft:grindstone", "minecraft:book"]
    ),
    (
        "grindable_armor",
        "Armor Recycling",
        "You may grind fully-repaired, unenchanted armor or horse armor to recover some materials.",
        [
            "minecraft:diamond_chestplate",
            "minecraft:netherite_horse_armor"
        ]
    ),
    (
        "kilns",
        "The Kiln",
        "A kiln is like a blast furnace, but for glass, glazed terracotta, and bricks.",
        ["strangely_charming:kiln"]
    ),
    (
        "lumberjacks",
        "The Table Saw",
        "Cuts logs and planks into wood products. A villager can also accept the role of lumberjack and offer some unique wood-related trades.",
        ["strangely_charming:woodcutter"]
    ),
    (
        "decorative_amethyst",
        "Decorative Amethyst",
        "Put an amethyst block in a stonecutter to choose from eight cuts. You can also craft four blocks into four amethyst bricks, then craft bricks into tiles and other patterns.",
        ["strangely_charming:amethyst_4", "strangely_charming:amethyst_1", "strangely_charming:amethyst_2", "strangely_charming:amethyst_3", "strangely_charming:amethyst_5", "strangely_charming:amethyst_6", "strangely_charming:amethyst_7", "strangely_charming:amethyst_8"]
    ),
    (
        "rune_circle_repair",
        "Restoration Ruby",
        "Use a ruby on a surviving carved rune to rebuild a damaged natural circle. The ritual clears the site and restores all eight original glyphs. There is no crafting recipe.",
        ["strangely_charming:restoration_ruby"]
    ),
    (
        "runebreaker",
        "Runebreaker",
        "Rune stones are nearly unbreakable! You need this special tool to destroy them. Doing so will yield some rune dust. If every original rune in a circle is destroyed, the circle is lost for good.",
        ["strangely_charming:runebreaker"]
    ),
    (
        "scrollkeepers",
        "The Scrollkeeper",
        "A nearby villager will become a quest-giving NPC! Trade for scrolls to begin quests, check progress with {scrollkeepers}, and sneak-use the completed scroll on any Scrollkeeper to collect your reward.",
        ["strangely_charming:writing_desk"]
    ),
    (
        "cycle_villager_trades",
        "Cycle Trades",
        "Click {cycle_villager_trades} at a new trader to reroll the offers. You no longer have to break their job block!",
        []
    ),
    (
        "villagers_follow_emerald_blocks",
        "Leading Villagers",
        "Hold an Emerald Block in either hand to lead nearby villagers the same way animals can be led with wheat.",
        ["minecraft:emerald_block"]
    ),
    (
        "beekeepers",
        "The Beekeeper",
        "A villager near a beehive may become a beekeeper with new trades!",
        ["minecraft:beehive"]
    ),
    (
        "poison_apple",
        "Poison Apples",
        "A safer alternative to the first step of villager curing. Give a villager the poison apple to instantly transform them into a zombie villager!",
        ["strangely_charming:poison_apple"]
    ),
    (
        "moonstones",
        "Moonstone",
        "You can find 16 variations of colorful decorative moonstone in The End. Look for large spherical geodes to crack open.",
        ["strangely_charming:moonstone_purple"]
    ),
    (
        "enchanting_totems",
        "Experience Storage",
        "Sneak-use the totem of enchanting to store your XP, then use it normally to release that XP back. These can be found in End cities.",
        ["strangely_charming:totem_of_enchanting"]
    ),
    (
        "rune_circles",
        "Rune Circles",
        "These mysterious stone circles can transport you around the world. Throwing pearls at the rune faces has a chance to teleport you and teach you its glyph. Ender pearls only work rarely and carry risk. Rune pearls work far better.",
        []
    ),
    (
        "coral_squids",
        "Coral Squids",
        "Look for these cute pink squids swimming near coral in warm oceans.",
        ["strangely_charming:coral_squid_spawn_egg"]
    ),
    (
        "raid_horns",
        "Retreat Horn",
        "Defeat raid captains for a chance to find a Retreat Horn. Blow it to call off a nearby raid. It recharges in ten minutes.",
        ["strangely_charming:raid_horn"]
    ),
    (
        "fullbright",
        "Night Vision",
        "Touching this journal allows you to see in the dark! Press {fullbright} to toggle night vision.",
        []
    ),
    (
        "inventory_totem",
        "Inventory Totem",
        "You no longer need to hold a totem in your hand for it to work. It can be anywhere in your main bag.",
        ["minecraft:totem_of_undying"]
    ),
    (
        "protect_pets",
        "Pet Protection",
        "Your own tamed animals are protected from your accidental melee attacks and direct projectile hits.",
        []
    ),
    (
        "ender_pearl_blocks",
        "Ender Pearl Block",
        "Eating chorus fruit near one makes you always appear on top of it.",
        [
            "strangely_charming:ender_pearl_block",
            "minecraft:chorus_fruit"
        ]
    ),
    (
        "crying_obsidian_frames",
        "Crying Obsidian in Portals",
        "You may now use crying obsidian in place of obsidian for nether portals!",
        ["minecraft:crying_obsidian"]
    ),
];
pub(super) fn lookup(id: &str) -> Option<usize> {
    let family = if id.starts_with("strangely_charming:moonstone_") {
        Some("moonstones")
    } else if id.starts_with("strangely_charming:scroll_") {
        Some("scrollkeepers")
    } else if id.starts_with("strangely_charming:runic_amethyst_") {
        return PAGES
            .iter()
            .position(|p| p.title == "Rune Portals" && p.enabled());
    } else if id.starts_with("strangely_charming:runestone_") {
        Some("rune_circles")
    } else {
        None
    };
    PAGES.iter().position(|p| {
        (p.items.contains(&id) || Some(p.key) == family)
            && p.enabled()
            && (p.key != "stackables"
                || horizon_mods::evanders_config::enabled(if id.contains("potion") {
                    "stackable_potions"
                } else if id.ends_with("book") {
                    "stackable_enchanted_books"
                } else {
                    "stackable_stews"
                }))
    })
}
