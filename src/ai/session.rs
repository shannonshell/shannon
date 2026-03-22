use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::shell::config_dir;

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub struct Session {
    pub id: String,
    pub messages: Vec<Message>,
    session_dir: PathBuf,
}

impl Session {
    pub fn new() -> Self {
        let session_dir = config_dir().join("sessions");
        let _ = fs::create_dir_all(&session_dir);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            messages: Vec::new(),
            session_dir,
        }
    }

    pub fn add_user(&mut self, content: &str) {
        self.messages.push(Message {
            role: "user".to_string(),
            content: content.to_string(),
        });
    }

    pub fn add_assistant(&mut self, content: &str) {
        self.messages.push(Message {
            role: "assistant".to_string(),
            content: content.to_string(),
        });
    }

    pub fn save(&self) {
        let path = self.session_dir.join(format!("{}.jsonl", self.id));
        let lines: Vec<String> = self
            .messages
            .iter()
            .map(|m| serde_json::to_string(m).unwrap_or_default())
            .collect();
        let _ = fs::write(path, lines.join("\n") + "\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new();
        assert!(!session.id.is_empty());
        assert!(session.messages.is_empty());
    }

    #[test]
    fn test_session_messages() {
        let mut session = Session::new();
        session.add_user("list files");
        session.add_assistant("ls -la");
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[0].content, "list files");
        assert_eq!(session.messages[1].role, "assistant");
        assert_eq!(session.messages[1].content, "ls -la");
    }

    #[test]
    fn test_session_save() {
        let mut session = Session::new();
        session.add_user("hello");
        session.add_assistant("world");
        session.save();
        // Verify file exists
        let path = session
            .session_dir
            .join(format!("{}.jsonl", session.id));
        assert!(path.exists());
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("hello"));
        assert!(contents.contains("world"));
        // Clean up
        let _ = std::fs::remove_file(path);
    }
}
