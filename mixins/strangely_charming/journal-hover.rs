fn hook(game: &crate::game::Game<'_>, stack: &ItemStack) {
    crate::game::sc_chat_items::hover(stack);
    game.sc_hover_item(&stack.item);
}
