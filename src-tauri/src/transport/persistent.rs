use crate::transport::request::ImageAttachment;
use log::info;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::sync::Mutex as TokioMutex;

/// A persistent CLI session that stays alive across multiple messages.
pub struct PersistentSession {
    stdin: Arc<TokioMutex<ChildStdin>>,
    pub provider: String,
}

impl PersistentSession {
    pub fn new(stdin: ChildStdin, provider: String) -> Self {
        Self {
            stdin: Arc::new(TokioMutex::new(stdin)),
            provider,
        }
    }

    /// Get a clone of the stdin Arc for async writes from other contexts.
    pub fn clone_stdin(&self) -> Arc<TokioMutex<ChildStdin>> {
        self.stdin.clone()
    }

    /// Write a message using a shared stdin reference.
    pub async fn write_to(
        stdin: &Arc<TokioMutex<ChildStdin>>,
        prompt: &str,
        images: &[ImageAttachment],
    ) -> Result<(), std::io::Error> {
        let msg = build_user_message(prompt, images);
        let msg_str = serde_json::to_string(&msg).unwrap_or_default();
        let mut s = stdin.lock().await;
        s.write_all(msg_str.as_bytes()).await?;
        s.write_all(b"\n").await?;
        s.flush().await?;
        info!(
            "PersistentSession: wrote message ({} bytes, {} images)",
            msg_str.len(),
            images.len()
        );
        Ok(())
    }
}

/// Build the VS Code extension-compatible stream-json user message.
pub fn build_user_message(prompt: &str, images: &[ImageAttachment]) -> serde_json::Value {
    let mut content = vec![serde_json::json!({
        "type": "text",
        "text": prompt,
    })];
    for img in images {
        content.push(serde_json::json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": img.mime_type,
                "data": img.data,
            }
        }));
    }
    serde_json::json!({
        "type": "user",
        "uuid": uuid::Uuid::new_v4().to_string(),
        "session_id": "",
        "message": {
            "role": "user",
            "content": content,
        },
        "parent_tool_use_id": null,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_user_message_text_only() {
        let msg = build_user_message("hello", &[]);
        assert_eq!(msg["type"], "user");
        assert_eq!(msg["message"]["role"], "user");
        let content = msg["message"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "hello");
    }

    #[test]
    fn build_user_message_with_image() {
        let images = vec![ImageAttachment {
            data: "abc123".to_string(),
            mime_type: "image/png".to_string(),
            name: "test.png".to_string(),
        }];
        let msg = build_user_message("describe", &images);
        let content = msg["message"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[1]["type"], "image");
        assert_eq!(content[1]["source"]["media_type"], "image/png");
        assert_eq!(content[1]["source"]["data"], "abc123");
    }

    #[test]
    fn build_user_message_has_uuid() {
        let msg = build_user_message("hi", &[]);
        assert!(msg["uuid"].is_string());
        assert!(!msg["uuid"].as_str().unwrap().is_empty());
    }
}
