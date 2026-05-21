pub mod client;
pub mod compiler;
pub mod prompts;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider_type", rename_all = "snake_case")]
pub enum LlmProvider {
    OpenAi {
        api_key: String,
        model: Option<String>,
        base_url: Option<String>,
    },
    Anthropic {
        api_key: String,
        model: Option<String>,
    },
    Ollama {
        base_url: String,
        model: String,
    },
}

pub use client::LlmClient;
pub use compiler::LlmCompiler;
