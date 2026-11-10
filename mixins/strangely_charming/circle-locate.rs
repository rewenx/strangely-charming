// Use the parent set's own spread or concentric rings. Confirm the marker in the
// actual start cache so old chunks without a circle are skipped by /locate.
fn hook(
    this: &mut GeneratorWorker,
    names: &[String],
    from: [i32; 3],
    radius: i32,
    skip: &dyn Fn(&str, ChunkPos) -> bool,
) -> core::ops::ControlFlow<Option<(String, [i32; 3])>> {
    use crate::worldgen::structure::sc_circle_sources::{CIRCLES, FREE};
    let marker = |name: &str| CIRCLES.iter().any(|&(_, id)| id == name);
    if !names.iter().any(|n| marker(n)) {
        return core::ops::ControlFlow::Continue(());
    }
    let others: Vec<_> = names.iter().filter(|n| !marker(n)).cloned().collect();
    let mut best = if others.is_empty() {
        None
    } else {
        this.pipeline
            .locate(&mut this.state, &others, from, radius, skip)
    };
    if this
        .pipeline
        .data()
        .structure_resources
        .as_ref()
        .is_some_and(|raw| raw.sets.contains_key(FREE))
    {
        let checker = std::cell::RefCell::new(
            OverworldGenerator {
                pipeline: this.pipeline.clone(),
                noise_clamped: None,
            }
            .worker(),
        );
        let distance = |p: [i32; 3]| {
            (i64::from(p[0]) - i64::from(from[0])).pow(2)
                + (i64::from(p[2]) - i64::from(from[2])).pow(2)
        };
        for (parent, id) in CIRCLES {
            if !names.iter().any(|n| n == id) {
                continue;
            }
            let exclude = |_: &str, c: ChunkPos| {
                skip(id, c)
                    || !checker
                        .borrow_mut()
                        .structure_starts(c)
                        .iter()
                        .any(|v| v.name == id)
            };
            if let Some((_, p)) =
                this.pipeline
                    .locate(&mut this.state, &[parent.into()], from, radius, &exclude)
                && best
                    .as_ref()
                    .is_none_or(|(_, old)| distance(p) < distance(*old))
            {
                best = Some((id.into(), p));
            }
        }
    }
    core::ops::ControlFlow::Break(best)
}
