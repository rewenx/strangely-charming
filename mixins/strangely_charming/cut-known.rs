fn hook(_done: (), menu: &mut Menu, env: &Env) {
    if menu.evanders_is_table_saw() {
        return;
    }
    let mask = menu.sc_cut_glyphs();
    menu.stone.recipes.retain(|&i| {
        crate::stations::sc_glyphs::id(&env.station_recipes.stonecutting[i].result.item)
            .is_none_or(|g| mask & (1 << g) != 0)
    });
}
