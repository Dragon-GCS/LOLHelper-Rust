pub mod champ_select;
pub mod chat;
pub mod game_flow;
pub mod matchmaking;

use serde::Deserialize;
use serde_json::Value;

use super::events::{
    champ_select::ChampSelectData,
    chat::{ChatConversation, chat_conversation_deserializer},
    game_flow::GameFlowSession,
    matchmaking::MatchMaking,
};

#[cfg(feature = "debug_events")]
pub(crate) const SUBSCRIBED_EVENT: [&str; 1] = ["OnJsonApiEvent"];
#[cfg(not(feature = "debug_events"))]
pub(crate) const SUBSCRIBED_EVENT: [&str; 4] = [
    "OnJsonApiEvent_lol-gameflow_v1_session",
    "OnJsonApiEvent_lol-lobby-team-builder_v1_matchmaking",
    "OnJsonApiEvent_lol-champ-select_v1_session",
    "OnJsonApiEvent_lol-chat_v1_conversations",
];

#[derive(Debug, Deserialize, PartialEq)]
pub enum EventType {
    Update,
    Delete,
    Create,
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
    #[serde(rename = "/lol-lobby-team-builder/champ-select/v1/current-champion")]
    CurrentChampion {
        #[serde(rename = "eventType")]
        event_type: EventType,
        data: u16, // ChampionId
    },
    #[serde(deserialize_with = "chat_conversation_deserializer")]
    #[serde(untagged)]
    ChatConversation(ChatConversation),

    #[serde(untagged)]
    Other(Value),
}
