fn hook(stack: &ItemStack, life: &mut i32, _ret: ()) {
    if stack.item.as_ref() == crate::game::sc_travel::BOOK
        && stack
            .data()
            .and_then(|d| d.other("minecraft:custom_data"))
            .is_some_and(|t| matches!(t,horizon_world::nbt::Tag::Compound(c) if !c.is_empty()))
    {
        *life = (*life).max(6000);
    }
}
