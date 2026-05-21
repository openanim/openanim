use crate::LlmProvider;
use crate::client::LlmClient;
use crate::prompts;
use anyhow::{Result, anyhow};
use scene_ir::scene::Scene;
use scene_ir::validation::validate_scene;

pub struct LlmCompiler {
    provider: LlmProvider,
    client: LlmClient,
}

impl LlmCompiler {
    pub fn new(provider: LlmProvider) -> Self {
        Self {
            provider,
            client: LlmClient::new(),
        }
    }

    /// Compiles a user natural language request from scratch into a fully validated Scene IR.
    pub async fn compile_scene(&self, user_prompt: &str) -> Result<Scene> {
        let system_prompt = prompts::get_system_prompt();
        let current_user_prompt = user_prompt.to_string();
        let mut attempts = 0;
        let max_attempts = 3;

        let mut bad_json = String::new();
        let mut last_error = String::new();

        while attempts < max_attempts {
            let prompt_to_send = if attempts == 0 {
                current_user_prompt.clone()
            } else {
                prompts::get_repair_prompt(&bad_json, &last_error)
            };

            let response_text = self
                .client
                .query(&self.provider, system_prompt, &prompt_to_send)
                .await?;
            let cleaned_json = clean_json_string(&response_text);

            match serde_json::from_str::<Scene>(&cleaned_json) {
                Ok(scene) => {
                    let validation_errors = validate_scene(&scene);
                    if validation_errors.is_empty() {
                        return Ok(scene);
                    } else {
                        let error_msg = validation_errors
                            .iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<String>>()
                            .join("\n");
                        bad_json = cleaned_json;
                        last_error = format!(
                            "JSON schema is valid but violated referential integrity/validation constraints:\n{}",
                            error_msg
                        );
                    }
                }
                Err(e) => {
                    bad_json = cleaned_json;
                    last_error = format!("JSON deserialization error: {}", e);
                }
            }

            attempts += 1;
        }

        Err(anyhow!(
            "Failed to compile scene after {} attempts. Last error: {}\nJSON tried:\n{}",
            max_attempts,
            last_error,
            bad_json
        ))
    }

    /// Patches an existing Scene IR to apply semantic modifications requested by the user.
    pub async fn patch_scene(
        &self,
        current_scene: &Scene,
        modification_prompt: &str,
    ) -> Result<Scene> {
        let current_scene_json = serde_json::to_string_pretty(current_scene)?;
        let system_prompt = prompts::get_system_prompt();
        let user_prompt = prompts::get_patch_prompt(&current_scene_json, modification_prompt);

        let mut attempts = 0;
        let max_attempts = 3;

        let mut bad_json = String::new();
        let mut last_error = String::new();
        let mut prompt_to_send = user_prompt.clone();

        while attempts < max_attempts {
            if attempts > 0 {
                prompt_to_send = prompts::get_repair_prompt(&bad_json, &last_error);
            }

            let response_text = self
                .client
                .query(&self.provider, system_prompt, &prompt_to_send)
                .await?;
            let cleaned_json = clean_json_string(&response_text);

            match serde_json::from_str::<Scene>(&cleaned_json) {
                Ok(scene) => {
                    let validation_errors = validate_scene(&scene);
                    if validation_errors.is_empty() {
                        return Ok(scene);
                    } else {
                        let error_msg = validation_errors
                            .iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<String>>()
                            .join("\n");
                        bad_json = cleaned_json;
                        last_error = format!("Validation error during patch:\n{}", error_msg);
                    }
                }
                Err(e) => {
                    bad_json = cleaned_json;
                    last_error = format!("JSON deserialization error during patch: {}", e);
                }
            }

            attempts += 1;
        }

        Err(anyhow!(
            "Failed to patch scene after {} attempts. Last error: {}\nJSON tried:\n{}",
            max_attempts,
            last_error,
            bad_json
        ))
    }
}

/// Helper function to clean markdown backticks from JSON responses if returned by LLMs.
fn clean_json_string(s: &str) -> String {
    let mut cleaned = s.trim().to_string();
    if cleaned.starts_with("```") {
        if let Some(first_newline) = cleaned.find('\n') {
            cleaned = cleaned[first_newline..].to_string();
        }
        if cleaned.ends_with("```") {
            cleaned = cleaned[..cleaned.len() - 3].to_string();
        }
    }
    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_json_string() {
        let input = "```json\n{\n  \"id\": \"test\"\n}\n```";
        let output = clean_json_string(input);
        assert_eq!(output, "{\n  \"id\": \"test\"\n}");

        let input_no_type = "```\n{\n  \"id\": \"test\"\n}\n```";
        let output_no_type = clean_json_string(input_no_type);
        assert_eq!(output_no_type, "{\n  \"id\": \"test\"\n}");
    }
}
