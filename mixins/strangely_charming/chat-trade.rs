use horizon_client::items::ItemStack;
impl super::GuestMerchant {
    pub(in crate::game) fn sc_find_link(
        &self,
        pred: &dyn Fn(&ItemStack) -> bool,
    ) -> Option<&ItemStack> {
        self.slots.iter().flatten().find(|s| pred(s))
    }
}
