// Trade identity and eligibility use native offer objects, never registry-ID ranges.
use super::{MerchantOffer, Session};

pub fn fresh(s: &Session) -> bool {
    s.villager
        && s.level == 1
        && s.xp == 0
        && !s.last_traded
        && !s.offers.is_empty()
        && s.offers.iter().all(|o| o.uses == 0)
}

pub fn book_first(offers: &mut [MerchantOffer]) {
    if let Some(i) = offers.iter().position(|o| {
        o.result.item.as_ref() == "minecraft:enchanted_book"
            || !o.result.stored_enchantments().is_empty()
    }) {
        offers[..=i].rotate_right(1);
    }
}
