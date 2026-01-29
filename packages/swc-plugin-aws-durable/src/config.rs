use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfig {
    #[serde(default = "default_mode")]
    pub mode: TransformMode,
    #[serde(default = "default_package_name")]
    pub package_name: String,
    #[serde(default = "default_env_prefix")]
    pub env_prefix: String,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            mode: TransformMode::Workflow,
            package_name: default_package_name(),
            env_prefix: default_env_prefix(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransformMode {
    Workflow,
    Client,
}

fn default_mode() -> TransformMode {
    TransformMode::Workflow
}

fn default_package_name() -> String {
    "@bento/aws-durable".to_string()
}

fn default_env_prefix() -> String {
    "WORKFLOW_".to_string()
}
