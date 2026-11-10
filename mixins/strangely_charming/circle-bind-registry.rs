fn hook(reach: i32, data: &Arc<GeneratorData>, bindings: &mut FeatureBlockBindings<'static>) -> i32 {
    bindings.sc_bind_circle_biomes(data);
    reach
}
