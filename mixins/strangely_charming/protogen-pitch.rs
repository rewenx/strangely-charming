fn hook(m: &mut Mob) -> core::ops::ControlFlow<f32> {
    if m.kind.id == "strangely_charming:protogen" && m.is_baby() {
        use horizon_core::random::RandomSource;
        return core::ops::ControlFlow::Break(1. + (m.random.next_float() - m.random.next_float()) * 0.2);
    }
    core::ops::ControlFlow::Continue(())
}
