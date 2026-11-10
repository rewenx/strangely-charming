fn hook(menu: &mut Menu) {
    menu.evanders_set_table_saw(evanders_saw::take_screen());
    menu.sc_set_cut_glyphs(crate::stations::sc_glyphs::client_mask());
}
