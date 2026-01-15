use std::sync::atomic::Ordering;

use crate::{CONTEXT, LcuClient, Result, events::EventType};
use serde::{Deserialize, Deserializer, de::Error};

#[derive(Debug, Deserialize)]
pub struct ChatConversation {
    pub id: String,
    pub event_type: EventType,
}

/// Deserialize ChatConversation event
pub(crate) fn chat_conversation_deserializer<'de, D>(
    deserializer: D,
) -> std::result::Result<ChatConversation, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    let prefix = "/lol-chat/v1/conversations/";

    if let Some(uri) = value.get("uri")
        && let uri = uri.as_str().unwrap()
        && uri.starts_with(prefix)
        && uri.ends_with("lol-champ-select.pvp.net")
    {
        let event_type = if let Some(event_type) = value.get("eventType") {
            EventType::deserialize(event_type).map_err(Error::custom)?
        } else {
            return Err(Error::missing_field("eventType"));
        };

        // 确保 URI 以指定前缀开头
        if !uri.starts_with(prefix) {
            return Err(Error::custom("URI does not start with the expected prefix"));
        }
        // 这里可以进一步解析 URI 以提取对话 ID
        let conversation_id = uri.strip_prefix(prefix).unwrap();

        return Ok(ChatConversation {
            id: conversation_id.to_owned(),
            event_type,
        });
    }

    Err(serde::de::Error::custom(
        "URI does not match chat conversation pattern",
    ))
}

impl LcuClient {
    pub(crate) async fn handle_chat_conversation_event(
        &self,
        data: ChatConversation,
    ) -> Result<()> {
        match data.event_type {
            EventType::Create => {
                *CONTEXT.conversation_id.write().unwrap() = data.id;
                CONTEXT.analysis_sent_flag.store(false, Ordering::Relaxed);
            }
            EventType::Delete => {
                CONTEXT.conversation_id.write().unwrap().clear();
            }
            _ => {}
        }
        Ok(())
    }
}
