use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Chat;
use rig::completion::Message as RigMessage;
use rig::providers::anthropic;

use crate::ai::prompt::PromptBuilder;
use crate::ai::session::Session;
use crate::ai_engine::AiConfig;

#[derive(Debug)]
pub enum AiError {
    NoApiKey(String),
    ApiError(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::NoApiKey(env_var) => {
                write!(f, "AI API key not found. Set {env_var} in your env.sh")
            }
            AiError::ApiError(msg) => write!(f, "AI error: {msg}"),
        }
    }
}

/// Translate a natural language question into a shell command.
pub fn translate_command(
    config: &AiConfig,
    session: &mut Session,
    shell_name: &str,
    cwd: &str,
    question: &str,
) -> Result<String, AiError> {
    let api_key_env = config
        .api_key_env
        .as_deref()
        .unwrap_or("ANTHROPIC_API_KEY");

    let api_key = std::env::var(api_key_env)
        .map_err(|_| AiError::NoApiKey(api_key_env.to_string()))?;

    let model = config
        .model
        .as_deref()
        .unwrap_or("claude-sonnet-4-20250514");

    let system_prompt = PromptBuilder::new()
        .base()
        .context(shell_name, cwd, std::env::consts::OS)
        .build();

    session.add_user(question);

    // Build conversation history for rig-core (all messages except the last user message)
    let mut history: Vec<RigMessage> = Vec::new();
    for msg in &session.messages[..session.messages.len() - 1] {
        match msg.role.as_str() {
            "user" => {
                history.push(RigMessage::user(msg.content.clone()));
            }
            "assistant" => {
                history.push(RigMessage::assistant(msg.content.clone()));
            }
            _ => {}
        }
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| AiError::ApiError(format!("failed to create runtime: {e}")))?;

    let response = rt.block_on(async {
        let client = anthropic::Client::from_val(api_key);
        let agent = client.agent(model).preamble(&system_prompt).build();
        agent.chat(question, history).await
    });

    match response {
        Ok(text) => {
            let command = strip_code_fences(&text);
            session.add_assistant(&command);
            session.save();
            Ok(command)
        }
        Err(e) => {
            session.messages.pop(); // remove failed user message
            Err(AiError::ApiError(e.to_string()))
        }
    }
}

/// Strip markdown code fences and leading/trailing whitespace from LLM output.
fn strip_code_fences(text: &str) -> String {
    let text = text.trim();

    // Strip ```bash\n...\n``` or ```\n...\n```
    if text.starts_with("```") && text.ends_with("```") {
        let inner = &text[3..text.len() - 3];
        let inner = if let Some(pos) = inner.find('\n') {
            &inner[pos + 1..]
        } else {
            inner
        };
        return inner.trim().to_string();
    }

    // Strip single backticks
    if text.starts_with('`') && text.ends_with('`') && !text.contains('\n') {
        return text[1..text.len() - 1].trim().to_string();
    }

    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_code_fences_plain() {
        assert_eq!(strip_code_fences("ls -la"), "ls -la");
    }

    #[test]
    fn test_strip_code_fences_backticks() {
        assert_eq!(strip_code_fences("`ls -la`"), "ls -la");
    }

    #[test]
    fn test_strip_code_fences_triple() {
        assert_eq!(strip_code_fences("```\nls -la\n```"), "ls -la");
    }

    #[test]
    fn test_strip_code_fences_with_language() {
        assert_eq!(strip_code_fences("```bash\nls -la\n```"), "ls -la");
    }

    #[test]
    fn test_strip_code_fences_whitespace() {
        assert_eq!(strip_code_fences("  ls -la  "), "ls -la");
    }
}
