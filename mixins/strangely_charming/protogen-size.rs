fn hook(m: &Mob) -> core::ops::ControlFlow<(f64, f64, f64)> {
    if m.kind.id == "strangely_charming:protogen" && m.is_baby() {
        // Its authored body is full size at every age. Preserve scale attributes and AI age.
        let scaled = |v: f64| f64::from(v as f32 * m.model_scale());
        return core::ops::ControlFlow::Break((scaled(m.kind.width), scaled(m.kind.height), scaled(m.kind.eye_height)));
    }
    core::ops::ControlFlow::Continue(())
}
