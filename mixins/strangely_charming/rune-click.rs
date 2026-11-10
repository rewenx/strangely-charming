fn hook(game: &mut crate::game::Game<'_>, pos: [i32; 3], canceled: &mut bool) {
    if !*canceled && game.sc_grave_break(pos){*canceled=true;return;}
    *canceled |= !game.creative && game.sc_rune_at(pos) && !game.sc_has_hammer();
}
