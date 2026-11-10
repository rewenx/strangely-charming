pub const NAMES: [&str; 26] = [
    "Un",
    "Pa",
    "Veh",
    "Gal",
    "Graph",
    "Or",
    "Ged",
    "Na",
    "Gon",
    "Gon",
    "Veh",
    "Ur",
    "Tal",
    "Drux",
    "Med",
    "Mals",
    "Ger",
    "Don",
    "Fam",
    "Gisg",
    "Van",
    "Van",
    "Double Van",
    "Pal",
    "Gon",
    "Ceph",
];

use std::{cell::RefCell, collections::HashMap};
const PREFIX: &str = "strangely_charming:runic_amethyst_";
pub fn id(item: &str) -> Option<u8> {
    item.strip_prefix(PREFIX)?
        .parse::<u8>()
        .ok()
        .filter(|&n| n < 26)
}
pub fn mask(ids: &[u8]) -> u32 {
    ids.iter()
        .filter(|&&g| g < 26)
        .fold(0, |m, &g| m | (1 << g))
}
thread_local! {static VIEW:RefCell<(u32,HashMap<u8,String>)>=RefCell::default();}
pub fn update(learned: &[u8], names: &HashMap<u8, String>) {
    VIEW.with_borrow_mut(|v| *v = (mask(learned), names.clone()));
}
pub fn clear() {
    VIEW.with_borrow_mut(|v| *v = Default::default());
}
pub fn client_mask() -> u32 {
    VIEW.with_borrow(|v| v.0)
}
pub fn personal_name(item: &str) -> Option<String> {
    let glyph = id(item)?;
    VIEW.with_borrow(|v| {
        v.1.get(&glyph)
            .map(|name| format!("Runic Amethyst ({name})"))
    })
}
