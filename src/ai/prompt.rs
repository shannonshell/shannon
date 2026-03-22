/// Builds the system prompt for AI mode, composed from sections.
pub struct PromptBuilder {
    sections: Vec<String>,
}

impl PromptBuilder {
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
        }
    }

    /// Add the base instructions for command translation.
    pub fn base(mut self) -> Self {
        self.sections.push(
            "You are a shell command translator. The user describes what they want to do \
             in plain English. You respond with exactly one shell command — nothing else. \
             No explanation, no markdown, no code fences, no backticks. Just the raw command.\n\
             \n\
             If the request is ambiguous, pick the most common interpretation.\n\
             If you truly cannot generate a command, respond with: # unable to generate command"
                .to_string(),
        );
        self
    }

    /// Add runtime context (shell name, cwd, OS).
    pub fn context(mut self, shell_name: &str, cwd: &str, os: &str) -> Self {
        self.sections.push(format!(
            "The user is in a {shell_name} shell on {os} in the directory: {cwd}"
        ));
        self
    }

    /// Build the final system prompt string.
    pub fn build(self) -> String {
        self.sections.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_builder() {
        let prompt = PromptBuilder::new()
            .base()
            .context("bash", "/home/user", "linux")
            .build();
        assert!(prompt.contains("shell command translator"));
        assert!(prompt.contains("bash"));
        assert!(prompt.contains("/home/user"));
        assert!(prompt.contains("linux"));
    }

    #[test]
    fn test_prompt_base_only() {
        let prompt = PromptBuilder::new().base().build();
        assert!(prompt.contains("shell command translator"));
        assert!(!prompt.contains("bash"));
    }
}
