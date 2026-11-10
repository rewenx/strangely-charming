fn hook(game: &crate::game::Game<'_>, button: i32, canceled: &mut bool, _ret: ()) {
    use winit::event::MouseButton as M;
    if *canceled {
        return;
    }
    let b = match button {
        0 => M::Left,
        1 => M::Right,
        2 => M::Middle,
        3 => M::Back,
        4 => M::Forward,
        n => {
            let Ok(n) = u16::try_from(n) else { return };
            M::Other(n)
        }
    };
    *canceled = game.sc_journal_mouse(button)
        || game.sc_share_input(&keybinds::mouse_name(b), false)
        || game.sc_lookup_press(&keybinds::mouse_name(b), false)
        || game.sc_cycle_input(&keybinds::mouse_name(b), false);
}
