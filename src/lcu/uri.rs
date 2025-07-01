pub struct LcuUri;

impl LcuUri {
    pub const ME: &'static str = "/lol-summoner/v1/current-summoner";
    pub const ACCEPT_GAME: &'static str = "/lol-matchmaking/v1/ready-check/accept";
    pub fn swap_champion(champion_id: u16) -> String {
        format!("/lol-champ-select/v1/session/bench/swap/{champion_id}")
    }
}
