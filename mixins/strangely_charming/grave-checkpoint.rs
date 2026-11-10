fn hook(g: &mut crate::game::Game<'_>, host: &mut LanHost) {
    g.sc_checkpoint_graves(host);
}
