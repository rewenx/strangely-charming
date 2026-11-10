fn hook(
    this: &StructureState,
    context: &mut impl StartContext,
    chunk: ChunkPos,
    starts: &mut Vec<StructureStart>,
) {
    crate::worldgen::structure::sc_circle_sources::attach(
        &this.registry,
        &this.resources,
        &this.bindings,
        context,
        chunk,
        starts,
    )
}
