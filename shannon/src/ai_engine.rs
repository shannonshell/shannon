use std::io::Write;

use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Chat;
use rig::completion::Message as RigMessage;
use rig::providers::anthropic;

use crate::ai::session::Session;
/// Configuration for AI mode.
#[derive(serde::Deserialize, Default, Clone)]
pub struct AiConfig {
    /// LLM provider (default: "anthropic")
    pub provider: Option<String>,
    /// Model name (default: "claude-sonnet-4-20250514")
    pub model: Option<String>,
    /// Environment variable name for the API key (default: "ANTHROPIC_API_KEY")
    pub api_key_env: Option<String>,
}
use crate::shell::ShellState;
use crate::shell_engine::ShellEngine;

pub struct AiEngine {
    config: AiConfig,
    session: Session,
    last_state: ShellState,
    runtime: tokio::runtime::Runtime,
}

impl AiEngine {
    pub fn new(config: AiConfig) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for AI engine");

        AiEngine {
            config,
            session: Session::new(),
            last_state: ShellState::from_current_env(),
            runtime,
        }
    }

    fn system_prompt(&self) -> String {
        format!(
            "You are a helpful assistant. The user is working in a terminal shell \
             on {} in the directory: {}.\n\n\
             Answer questions, explain commands, help with code. Be concise. \
             Use markdown formatting when helpful.",
            std::env::consts::OS,
            self.last_state.cwd.to_string_lossy()
        )
    }

    fn chat(&mut self, message: &str) -> Result<String, String> {
        let api_key_env = self
            .config
            .api_key_env
            .as_deref()
            .unwrap_or("ANTHROPIC_API_KEY");

        // Check shell state first (set via env.sh), then process env
        let api_key = self
            .last_state
            .env
            .get(api_key_env)
            .cloned()
            .or_else(|| std::env::var(api_key_env).ok())
            .ok_or_else(|| format!("AI API key not found. Set {api_key_env} in your env.sh"))?;

        let model = self
            .config
            .model
            .as_deref()
            .unwrap_or("claude-sonnet-4-20250514");

        let system_prompt = self.system_prompt();

        self.session.add_user(message);

        // Build conversation history (all messages except the last user message)
        let mut history: Vec<RigMessage> = Vec::new();
        for msg in &self.session.messages[..self.session.messages.len() - 1] {
            match msg.role.as_str() {
                "user" => history.push(RigMessage::user(msg.content.clone())),
                "assistant" => history.push(RigMessage::assistant(msg.content.clone())),
                _ => {}
            }
        }

        let response = self.runtime.block_on(async {
            let client = anthropic::Client::from_val(api_key);
            let agent = client.agent(model).preamble(&system_prompt).build();
            agent.chat(message, history).await
        });

        match response {
            Ok(text) => {
                self.session.add_assistant(&text);
                self.session.save();
                Ok(text)
            }
            Err(e) => {
                self.session.messages.pop(); // remove failed user message
                Err(format!("AI error: {e}"))
            }
        }
    }
}

impl ShellEngine for AiEngine {
    fn inject_state(&mut self, state: &ShellState) {
        self.last_state = state.clone();
    }

    fn execute(&mut self, command: &str) -> ShellState {
        eprint!("  Thinking...");
        std::io::stderr().flush().ok();

        match self.chat(command) {
            Ok(response) => {
                eprint!("\r\x1b[K");
                eprintln!("{response}");
            }
            Err(e) => {
                eprint!("\r\x1b[K");
                eprintln!("  shannon: {e}");
            }
        }

        // AI chat doesn't change shell state — return it unchanged
        self.last_state.clone()
    }
}
