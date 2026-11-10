fn hook(id: &str) -> core::ops::ControlFlow<String> {
    if let Some(name) = crate::stations::sc_glyphs::personal_name(id) {
        core::ops::ControlFlow::Break(name)
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
