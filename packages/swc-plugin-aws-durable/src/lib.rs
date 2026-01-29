pub mod codegen;
pub mod collector;
pub mod config;
pub mod directive;
pub mod transform;

#[cfg(test)]
mod tests;

use swc_core::ecma::ast::Program;
use swc_core::ecma::visit::{VisitMut, VisitMutWith, VisitWith};
use swc_core::plugin::plugin_transform;

use collector::Collector;
use config::PluginConfig;
use transform::WorkflowTransformer;

struct TransformPass {
    config: PluginConfig,
}

impl VisitMut for TransformPass {
    fn visit_mut_module(&mut self, module: &mut swc_core::ecma::ast::Module) {
        // Pass 1: Collect info
        let mut collector = Collector::new(&self.config);
        module.visit_with(&mut collector);

        // Pass 2: Transform
        let mut transformer = WorkflowTransformer::new(self.config.clone(), collector.info);
        transformer.visit_mut_module(module);
    }
}

#[plugin_transform]
pub fn process_transform(
    mut program: Program,
    metadata: swc_core::plugin::metadata::TransformPluginProgramMetadata,
) -> Program {
    let config: PluginConfig = metadata
        .get_transform_plugin_config()
        .and_then(|json_str| serde_json::from_str(&json_str).ok())
        .unwrap_or_default();

    let mut pass = TransformPass { config };
    program.visit_mut_with(&mut pass);
    program
}
