fn hook(hub: &mut Hub, i: usize) {
    if hub.guests[i].recipes_unlocked.iter().any(String::is_empty) {
        hub.guests[i].recipes_unlocked.retain(|id| !id.is_empty());
        hub.send_initial_recipe_book(i);
    }
}
