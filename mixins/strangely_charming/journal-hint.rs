fn hook(game: &crate::game::Game<'_>, stack: &ItemStack, lines: &mut Vec<(Text, u32)>) {
    if let Some(hint) = game.sc_lookup_hint(&stack.item) {
        lines.push((Text::literal(hint), crate::game::rgba(184, 154, 224, 255)));
    }
}
