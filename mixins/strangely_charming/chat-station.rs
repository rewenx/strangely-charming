use horizon_client::items::ItemStack;
impl super::GuestStation {
    pub(in crate::game) fn sc_find_link(
        &self,
        pred: &dyn Fn(&ItemStack) -> bool,
    ) -> Option<&ItemStack> {
        (0..self.menu.slot_count()).find_map(|i| self.menu.slot(i).filter(|s| pred(s)))
    }
}
