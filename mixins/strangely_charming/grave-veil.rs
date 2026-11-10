// Each Horizon world server runs on its own thread. Keep the veil outside the
// potion list: milk, an existing invisibility potion and its remaining time
// keep their normal meaning. A connection may only see its own UUID's veil.
use std::{cell::RefCell, collections::HashMap, path::{Path, PathBuf}};
type Actor = (u128, Option<u32>);
thread_local! {
    static ACTIVE: RefCell<HashMap<Actor, (PathBuf, i64)>> = RefCell::new(HashMap::new());
}
pub fn start(root: &Path, who: u128, conn: Option<u32>, now: i64) {
    ACTIVE.with_borrow_mut(|active| {
        active.insert((who, conn), (root.to_owned(), now.saturating_add(600)));
    });
}
pub fn stop(who: u128, conn: Option<u32>) {
    ACTIVE.with_borrow_mut(|active| { active.remove(&(who, conn)); });
}
pub fn stop_owner(root: &Path, who: u128) {
    ACTIVE.with_borrow_mut(|all| all.retain(|(id, _), (world, _)| *id != who || world != root));
}
pub fn active(root: &Path, who: u128, conn: Option<u32>, now: i64) -> bool {
    ACTIVE.with_borrow_mut(|a| {
        let key = (who, conn);
        match a.get(&key) {
            Some((world, end)) if world == root && now < *end => true,
            Some(_) => { a.remove(&key); false },
            None => false,
        }
    })
}
pub fn guest_body(who: u128, conn: u32) -> bool {
    ACTIVE.with_borrow(|a| a.contains_key(&(who, Some(conn))))
}
