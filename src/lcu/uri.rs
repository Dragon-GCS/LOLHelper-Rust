pub struct LcuUri;

impl LcuUri {
    pub const ME: &'static str = "/lol-summoner/v1/current-summoner";
    pub const ACCEPT_GAME: &'static str = "/lol-matchmaking/v1/ready-check/accept";
    pub const CONVERSATIONS: &'static str = "/lol-chat/v1/conversations";
    pub const OWNED_CHAMPIONS: &'static str = "/lol-champions/v1/owned-champions-minimal";

    pub fn swap_champion(champion_id: u16) -> String {
        format!("/lol-champ-select/v1/session/bench/swap/{champion_id}")
    }
    pub fn conversation_message(conversation_id: &str) -> String {
        format!("{}/{}/messages", LcuUri::CONVERSATIONS, conversation_id)
    }
    pub fn summoners_by_puuid(puuid: &str) -> String {
        format!("/lol-summoner/v2/summoners/puuid/{puuid}")
    }
    pub fn matches(puuid: &str, begin: usize, end: usize) -> String {
        format!(
            "/lol-match-history/v1/products/lol/{puuid}/matches?begIndex={begin}&endIndex={end}"
        )
    }
    pub fn bp_champions(action: &str) -> String {
        format!("/lol-champ-select/v1/session/actions/{action}")
    }
}
