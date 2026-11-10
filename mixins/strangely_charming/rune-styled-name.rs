fn hook(stack: &ItemStack) -> core::ops::ControlFlow<crate::text::Text> {
    if stack.custom_name().is_none()
        && let Some(name) = crate::stations::sc_glyphs::personal_name(&stack.item)
    {
        return core::ops::ControlFlow::Break(crate::text::Text::literal(name));
    }
    core::ops::ControlFlow::Continue(())
}
