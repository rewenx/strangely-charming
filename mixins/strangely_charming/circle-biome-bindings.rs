use super::FeatureBlockBindings;
use crate::worldgen::{
    biome::BiomeId, biome_predicate::BiomePredicateBindings, generator::GeneratorData,
};
impl FeatureBlockBindings<'_> {
    pub(crate) fn sc_bind_circle_biomes(&mut self, data: &GeneratorData) {
        if self.biomes.is_none() {
            let names = data
                .biome_names()
                .iter()
                .enumerate()
                .map(|(i, name)| (name.to_string(), BiomeId(i as u32)))
                .collect();
            if let Ok(registry) = BiomePredicateBindings::new(&names, &data.biome_tags) {
                self.biomes = Some(std::sync::Arc::new(registry));
            }
        }
    }
}
