fn hook(game: &mut crate::game::Game<'_>, who: (u128, i32)) {
    game.sc_death_start(who.0);
}
