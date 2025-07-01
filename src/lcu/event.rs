use serde::{Deserialize, Deserializer};
#[cfg(debug_assertions)]
use serde_json::Value;

#[cfg(not(debug_assertions))]
pub(crate) const SUBSCRIBED_EVENT: [&str; 4] = [
    "lol-gameflow_v1_session",
    "lol-matchmaking_v1_ready-check",
    "lol-lobby-team-builder_v1_matchmaking",
    "lol-champ-select_v1_session",
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

#[derive(Default, Debug, Deserialize, PartialEq, Eq, Clone)]
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
    #[cfg(debug_assertions)]
    #[serde(untagged)]
    Other(Value),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectData {
    #[serde(deserialize_with = "deserialize_champion_ids")]
    pub bench_champions: Vec<u16>,
    #[serde(rename = "benchEnabled")]
    pub bench_enabled: bool,
    pub id: String,
    #[serde(rename = "myTeam")]
    pub my_team: Vec<ChampSelectPlayer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectPlayer {
    #[serde(default)]
    #[serde(rename = "cellId")]
    pub cell_id: u8,
    pub puuid: String,
    #[serde(rename = "summonerId")]
    pub summoner_id: u64,
    #[serde(rename = "championId")]
    pub champion_id: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFlowSession {
    pub phase: GamePhase,
    #[serde(rename = "gameData")]
    pub game_data: GameFlowGameData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameFlowGameData {
    pub game_id: u64,
    pub team_one: Vec<ChampSelectPlayer>,
    pub team_two: Vec<ChampSelectPlayer>,
}

#[derive(Debug, Deserialize)]
pub struct MatchMaking {
    #[serde(rename = "queueId")]
    pub queue_id: u16,
    #[serde(rename = "searchState")]
    pub search_state: String,
    #[serde(rename = "readyCheck")]
    pub ready_check: MatchMakingReadyCheck,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchMakingReadyCheck {
    pub state: String,
    pub timer: f32,
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
