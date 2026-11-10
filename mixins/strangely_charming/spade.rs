fn hook(
    game: &mut crate::game::Game<'_>,
    hit: Option<&Hit>,
) -> core::ops::ControlFlow<Result<bool>> {
    use core::ops::ControlFlow::{Break, Continue};
    use horizon_core::random::RandomSource;
    let Some(hit) = hit else { return Continue(()) };
    if game.remote.is_some()
        || game
            .held()
            .is_none_or(|s| s.item.as_ref() != "strangely_charming:spade")
        || !horizon_mods::evanders_config::enabled("spade")
    {
        return Continue(());
    }
    let mut candidates = [0usize; 9];
    let mut len = 0;
    for i in 0..9 {
        if game.stack_in(i).is_some_and(|s| {
            s.count > 0
                && s.item.as_ref() != "minecraft:air"
                && game.items().get(&s.item).is_some_and(|i| i.block.is_some())
        }) {
            candidates[len] = i;
            len += 1;
        }
    }
    if len == 0 {
        return Continue(());
    }
    let slot = candidates[game.world.random.next_int_bound(len as i32).unwrap_or(0) as usize];
    let source = game.stack_in_mut(slot).take();
    let spade = std::mem::replace(game.held_mut(), source);
    let targets = [hit.pos, hit.beside()];
    let states = targets.map(|[x, y, z]| game.world.block(x, y, z));
    let result = game.use_hand(Some(hit));
    if result.as_ref().is_ok_and(|ok| *ok) && game.lan_acting.is_some() {
        for (i, [x, y, z]) in targets.into_iter().enumerate() {
            let after = game.world.block(x, y, z);
            if after != states[i] {
                let sounds = game.res.facts().get(after).sounds.clone();
                let who = game.native_acting_id();
                game.native_play_sound_to(
                    who,
                    &sounds.place,
                    "block",
                    Some([x as f64 + 0.5, y as f64 + 0.5, z as f64 + 0.5]),
                    (sounds.volume + 1.) / 2.,
                    sounds.pitch * 0.8,
                );
                break;
            }
        }
    }
    let rem = std::mem::replace(game.held_mut(), spade);
    *game.stack_in_mut(slot) = rem;
    Break(result)
}
