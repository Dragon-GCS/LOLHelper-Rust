use std::{fmt::Display, sync::Arc, sync::atomic::Ordering};

use crate::lcu::Result;
use log::info;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use tokio::time::{Duration, sleep};

use crate::{
    context::{HelperContext, Summoner},
    lcu::LcuClient,
};

#[derive(Debug, Default)]
pub struct PlayerScore {
    name: String, // 玩家名称
    kda: f32,     // (击杀 + 助攻) / (死亡 + 1)
    dpm: f32,     // 分均伤害
    repeats: i8,  // 大于0表示连胜，否则连败
    wins: u32,    // 胜场
    total: u32,   // 总场次
}

impl PlayerScore {
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
}

impl Display for PlayerScore {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let streak_text = if self.repeats > 0 { "连胜" } else { "连败" };
        write!(
            f,
            "{}战绩信息：\n\
            kda={:.2}，分均伤害={:.2}\n\
            胜率={}/{}，{}{}",
            self.name,
            self.kda,
            self.dpm,
            self.wins,
            self.total,
            self.repeats.abs(),
            streak_text
        )
    }
}

#[derive(Debug, serde::Deserialize)]
struct Status {
    assists: u16,
    deaths: u16,
    kills: u16,
    win: bool,
    #[serde(rename = "totalDamageDealtToChampions")]
    total_damage_dealt_to_champions: u32,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Match {
    game_mode: String,
    // pub game_creation: u64, // 毫秒
    game_duration: u32, // 秒
    #[serde(
        deserialize_with = "deserialize_status_from_participants",
        rename = "participants"
    )]
    status: Status,
}

#[derive(Debug)]
pub struct Matches(Vec<Match>);
pub const MAX_MATCHES: usize = 20;

impl Matches {
    pub fn calculate_player_score(self, game_mode: &str) -> PlayerScore {
        let matches = self
            .0
            .into_iter()
            .filter(|m| m.game_mode == game_mode)
            .collect::<Vec<Match>>();

        let total = matches.len() as u32;
        if total == 0 {
            return PlayerScore::default();
        }
        let (mut kills, mut deaths, mut assists, mut wins, mut damage, mut duration) =
            (0, 0, 0, 0, 0, 0);
        for m in &matches {
            kills += m.status.kills;
            deaths += m.status.deaths;
            assists += m.status.assists;
            wins += m.status.win as u32;
            damage += m.status.total_damage_dealt_to_champions;
            duration += m.game_duration;
        }

        let (win, mut repeat) = (matches[0].status.win, 1);
        for m in &matches[1..] {
            if m.status.win != win {
                break;
            }
            repeat += 1;
        }

        PlayerScore {
            name: String::new(),
            kda: (kills + assists) as f32 / (deaths + 1) as f32,
            dpm: if duration > 0 {
                (damage as f32 / duration as f32) * 60.0 // 分均伤害
            } else {
                0.0
            },
            repeats: if win { repeat } else { -repeat },
            wins,
            total,
        }
    }
}

impl<'de> Deserialize<'de> for Matches {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 定义中间结构来匹配 JSON 层次结构
        #[derive(Deserialize)]
        struct GamesWrapper {
            games: Vec<Match>,
        }

        #[derive(Deserialize)]
        struct ObjectWrapper {
            games: GamesWrapper,
        }

        // 反序列化嵌套结构
        let wrapper = ObjectWrapper::deserialize(deserializer)?;
        Ok(Matches(wrapper.games.games))
    }
}

fn deserialize_status_from_participants<'de, D>(
    deserializer: D,
) -> std::result::Result<Status, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Participant {
        stats: Status,
    }

    let participants: Vec<Participant> = Vec::deserialize(deserializer)?;
    participants
        .into_iter()
        .next()
        .map(|p| p.stats)
        .ok_or_else(|| D::Error::custom("No participants found"))
}

impl LcuClient {
    async fn get_matches(&self, puuid: &str, begin: usize, num: usize) -> Result<Matches> {
        let end = begin + num - 1;
        let response = self
            .get(&format!(
                "/lol-match-history/v1/products/lol/{puuid}/matches?begIndex={begin}&endIndex={end}"
            ))
            .await?;
        Ok(response.json::<Matches>().await?)
    }

    pub async fn analyze_player(&self, puuid: &str, game_mode: &str) -> Result<PlayerScore> {
        let summoner = self
            .get(&format!("/lol-summoner/v2/summoners/puuid/{puuid}"))
            .await?
            .json::<Summoner>()
            .await?;
        let matches = self.get_matches(puuid, 0, MAX_MATCHES).await?;
        let mut score = matches.calculate_player_score(game_mode);
        score.set_name(&summoner.game_name);
        Ok(score)
    }

    pub(crate) async fn analyze_team_players(&self, ctx: Arc<HelperContext>) -> Result<()> {
        if ctx.analysis_sent_flag.load(Ordering::Relaxed)
            || ctx.game_mode.read().unwrap().is_empty()
            || ctx.conversation_id.read().unwrap().is_empty()
        {
            return Ok(());
        }

        let game_mode = { ctx.game_mode.read().unwrap().clone() };
        let conversation_id = { ctx.conversation_id.read().unwrap().clone() };
        info!("当前游戏模式: {game_mode}");
        let puuids = {
            ctx.my_team
                .read()
                .unwrap()
                .iter()
                .filter_map(|player| {
                    if player.puuid.is_empty() {
                        None
                    } else {
                        Some(player.puuid.clone())
                    }
                })
                .collect::<Vec<String>>()
        };
        for puuid in puuids {
            let player_score = self.analyze_player(&puuid, &game_mode).await?;
            let msg = format!("{player_score}");
            self.send_message(&conversation_id, &msg).await;
            sleep(Duration::from_secs(1)).await; // 避免发送消息过快
        }
        ctx.analysis_sent_flag.store(true, Ordering::Relaxed);
        Ok(())
    }
}
