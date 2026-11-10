use horizon_client::items::ItemStack;
impl super::GuestMount {
    pub(in crate::game) fn sc_find_link(
        &self,
        pred: &dyn Fn(&ItemStack) -> bool,
    ) -> Option<&ItemStack> {
        self.sent.as_ref()?.0.iter().flatten().find(|s| pred(s))
    }
}
