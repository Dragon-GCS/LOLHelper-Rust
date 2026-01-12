use log::info;

use crate::lcu::LcuClient;

#[derive(serde::Serialize)]
pub struct MessageBody {
    body: String,
    #[serde(rename = "type")]
    body_type: String,
}

impl MessageBody {
    pub fn message(message: &str) -> Self {
        MessageBody {
            body: message.to_string(),
            body_type: "chat".to_string(),
        }
    }
}

impl LcuClient {
    pub(crate) async fn send_message(&self, conversation_id: &str, message: &str) {
        let _ = self
            .post_json(
                &format!("/lol-chat/v1/conversations/{conversation_id}/messages"),
                &MessageBody::message(message),
            )
            .await
            .map(|_| {
                info!("发送消息到对话({conversation_id}):\n{message}");
            });
    }
}
