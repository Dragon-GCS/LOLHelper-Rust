use std::{
    sync::{LazyLock, atomic::Ordering},
    time::Duration,
};

use crate::{CONTEXT, LcuClient};
use log::error;
use regex::Regex;
use serde::{Deserialize, Deserializer, de::Error};
use tokio::time::sleep;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Chat,
    System,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageData {
    pub body: String,
    pub from_summoner_id: u64,
    pub from_puuid: String,
    #[serde(rename = "type")]
    pub message_type: MessageType,
}
#[derive(Debug, Deserialize)]
pub struct ChatConversation {
    pub conversation_id: String,
    pub data: ChatMessageData,
}

static PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^/lol-chat/v1/conversations/(.+lol-champ-select\.pvp\.net)/messages/.+").unwrap()
});

/// Deserialize ChatConversation event
pub(crate) fn chat_conversation_deserializer<'de, D>(
    deserializer: D,
) -> std::result::Result<ChatConversation, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    // 1. 提取 conversation ID
    let uri = value
        .get("uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::missing_field("uri"))?;

    let cap = PATTERN
        .captures(uri)
        .ok_or_else(|| Error::custom("URI does not match chat conversation pattern"))?;

    // 2. 检查 eventType 是否为 Create
    let event_type = value
        .get("eventType")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::missing_field("eventType"))?;

    if event_type != "Create" {
        return Err(Error::custom("Not a Create event"));
    }

    // 3. 反序列化 data 并返回
    let conversation_id = cap[1].to_string();
    let data_value = value
        .get("data")
        .ok_or_else(|| Error::missing_field("data"))?;
    let data = ChatMessageData::deserialize(data_value).map_err(Error::custom)?;

    Ok(ChatConversation {
        conversation_id,
        data,
    })
}

impl LcuClient {
    pub async fn handle_chat_conversation_event(&self, data: ChatConversation) {
        if !CONTEXT.auto_send_analysis.load(Ordering::Relaxed)
            || data.data.message_type != MessageType::System
            || data.data.body != "joined_room"
            || CONTEXT.game_mode.read().unwrap().is_empty()
            || *CONTEXT.game_mode.read().unwrap() == "TFT"
        {
            return;
        }
        let conversation_id = data.conversation_id;
        let game_mode = CONTEXT.game_mode.read().unwrap().clone();
        if let Ok(player_score) = self
            .analyze_player(&data.data.from_puuid, &game_mode)
            .await
            .inspect_err(|e| error!("战绩分析失败: {:?}", e))
        {
            self.send_message(&conversation_id, &format!("{player_score}"))
                .await;
            sleep(Duration::from_secs(1)).await; // 避免发送消息过快
        };
    }
}

#[test]
fn test_chat_conversation_deserializer() {
    let json_data = r#"
    {
        "data": {
            "body": "joined_room",
            "fromId": "puuid",
            "fromObfuscatedPuuid": "",
            "fromObfuscatedSummonerId": 0,
            "fromPid": "puuid",
            "fromPuuid": "puuid",
            "fromSummonerId": 1234,
            "id": "message_id",
            "isHistorical": false,
            "timestamp": "2026-01-18T03:03:37.872Z",
            "type": "system"
        },
        "eventType": "Create",
        "uri": "/lol-chat/v1/conversations/b564c626-9349-47f4-844e-5fce22a50d7a%40lol-champ-select.pvp.net/messages/message_id"
    }"#;

    // 直接通过 Event enum 来测试，这样会触发自定义反序列化器
    let mut deserializer = serde_json::Deserializer::from_str(json_data);
    let chat = chat_conversation_deserializer(&mut deserializer).expect("Deserialization failed");

    assert_eq!(
        chat.conversation_id,
        "b564c626-9349-47f4-844e-5fce22a50d7a%40lol-champ-select.pvp.net"
    );
    assert_eq!(chat.data.body, "joined_room");
    assert_eq!(chat.data.from_summoner_id, 1234);
    assert_eq!(chat.data.from_puuid, "puuid");
    assert_eq!(chat.data.message_type, MessageType::System);
}
