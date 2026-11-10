fn hook(m: &Mob) -> core::ops::ControlFlow<(Cow<'static, str>, Cow<'_, str>)> {
    let body = match m.kind.id.as_str() {
        "strangely_charming:protogen" => ("protogen#main", "strangely_charming:entity/protogen"),
        "strangely_charming:coral_squid" => ("squid#main", "strangely_charming:entity/coral_squid/brain"),
        _ => return core::ops::ControlFlow::Continue(()),
    };
    core::ops::ControlFlow::Break((body.0.into(), body.1.into()))
}
