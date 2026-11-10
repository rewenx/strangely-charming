fn hook(menu: &mut Menu, env: &Env, index: i32) -> core::ops::ControlFlow<()> {
    if !menu.evanders_is_table_saw() {
        let recipe = usize::try_from(index)
            .ok()
            .and_then(|i| menu.stone.recipes.get(i))
            .and_then(|&i| env.station_recipes.stonecutting.get(i));
        if let Some(glyph) = recipe.and_then(|r| crate::stations::sc_glyphs::id(&r.result.item))
            && menu.sc_cut_glyphs() & (1 << glyph) == 0
        {
            menu.data[0] = -1;
            menu.stone.used = None;
            menu.containers[RESULT][0] = None;
            return core::ops::ControlFlow::Break(());
        }
        return core::ops::ControlFlow::Continue(());
    }
    let recipes = crate::stations::evanders_saw::recipes();
    if std::ptr::eq(env.station_recipes, recipes) {
        return core::ops::ControlFlow::Continue(());
    }
    let saw = Env {
        station_recipes: recipes,
        ..*env
    };
    core::ops::ControlFlow::Break(menu.stone_setup_result(&saw, index))
}
