use serde::{Deserialize, Deserializer, de::Error};
#[cfg(debug_assertions)]
use serde_json::Value;

#[cfg(not(debug_assertions))]
pub(crate) const SUBSCRIBED_EVENT: [&str; 5] = [
    "lol-gameflow_v1_session",
    "lol-matchmaking_v1_ready-check",
    "lol-lobby-team-builder_v1_matchmaking",
    "lol-champ-select_v1_session",
    "lol-chat_v1_conversations",
];

#[derive(Debug, Deserialize)]
pub(super) struct EventMessage(
    u8,        // event code, 8
    String,    // event type, OnJsonEvent
    pub Event, // event data
);

#[derive(Debug, Deserialize)]
pub enum EventType {
    Update,
    Delete,
    Create,
}

#[derive(Default, Debug, Deserialize, PartialEq, Eq)]
pub enum GamePhase {
    ChampSelect,
    GameStart,
    InProgress,
    Lobby,
    Matchmaking,
    #[default]
    None,
    PreEndOfGame,
    ReadyCheck,
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "uri")]
pub enum Event {
    #[serde(rename = "/lol-gameflow/v1/session")]
    GameFlowSession {
        #[serde(rename = "eventType")]
        _event_type: EventType,
        data: GameFlowSession,
    },
    #[serde(rename = "/lol-matchmaking/v1/ready-check")]
    MatchmakingReadyCheck {
        #[serde(rename = "eventType")]
        _event_type: EventType,
        data: Option<MatchMakingReadyCheck>,
    },
    #[serde(rename = "/lol-lobby-team-builder/v1/matchmaking")]
    LobbyTeamBuilderMatchmaking {
        #[serde(rename = "eventType")]
        _event_type: EventType,
        data: Option<MatchMaking>,
    },
    #[serde(rename = "/lol-champ-select/v1/session")]
    ChampSelectSession {
        #[serde(rename = "eventType")]
        _event_type: EventType,
        data: ChampSelectData,
    },
    #[serde(deserialize_with = "chat_conversation_deserializer")]
    #[serde(untagged)]
    ChatConversation(ChatConversation),

    #[cfg(debug_assertions)]
    #[serde(untagged)]
    Other(Value),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectData {
    #[serde(deserialize_with = "deserialize_champion_ids")]
    pub bench_champions: Vec<u16>,
    pub bench_enabled: bool,
    pub id: String,
    pub my_team: Vec<ChampSelectPlayer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectPlayer {
    #[serde(default)]
    pub cell_id: u8,
    pub puuid: String,
    pub summoner_id: u64,
    pub champion_id: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFlowSession {
    pub phase: GamePhase,
    pub game_data: GameFlowGameData,
    pub map: Map,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFlowGameData {
    pub game_id: u64,
    pub team_one: Vec<ChampSelectPlayer>,
    pub team_two: Vec<ChampSelectPlayer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Map {
    pub game_mode: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchMaking {
    pub queue_id: u16,
    pub search_state: String,
    pub ready_check: MatchMakingReadyCheck,
}

#[derive(Debug, Deserialize)]

pub enum MatchReadyResponse {
    Accepted,
    Declined,
    None,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchMakingReadyCheck {
    pub player_response: MatchReadyResponse,
    pub timer: f32,
}

#[derive(Debug, Deserialize)]
pub struct ChatConversation {
    pub id: String,
    pub event_type: EventType,
}

/// Deserialize champion IDs from a JSON array of objects
fn deserialize_champion_ids<'de, D>(deserializer: D) -> Result<Vec<u16>, D::Error>
where
    D: Deserializer<'de>,
{
    // 先反序列化为中间结构
    #[derive(Deserialize)]
    struct ChampWrapper {
        #[serde(rename = "championId")]
        champion_id: u16,
    }

    // 然后提取 champion_id 字段值
    let wrappers = Vec::<ChampWrapper>::deserialize(deserializer)?;
    Ok(wrappers.into_iter().map(|w| w.champion_id).collect())
}

/// Deserialize ChatConversation event
fn chat_conversation_deserializer<'de, D>(deserializer: D) -> Result<ChatConversation, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    let prefix = "/lol-chat/v1/conversations/";

    if let Some(uri) = value.get("uri").and_then(|v| v.as_str()) {
        if uri.starts_with(prefix) && uri.ends_with("lol-champ-select.pvp.net") {
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
    }

    Err(serde::de::Error::custom(
        "URI does not match chat conversation pattern",
    ))
}
