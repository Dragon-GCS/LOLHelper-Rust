use std::fmt::Display;

use serde::de::Error;
use serde::{Deserialize, Deserializer};

#[derive(serde::Serialize)]
pub(crate) struct MessageBody {
    body: String,
    #[serde(rename = "type")]
    body_type: String,
}

impl MessageBody {
    pub(crate) fn message(message: &str) -> Self {
        MessageBody {
            body: message.to_string(),
            body_type: "chat".to_string(),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct PlayerScore {
    pub name: String,  // 玩家名称
    pub kda: f32,      // (击杀 + 助攻) / (死亡 + 1)
    pub dpm: f32,      // 分均伤害
    pub repeats: i8,   // 大于0表示连胜，否则连败
    pub win_rate: f32, // 胜率
}

impl PlayerScore {
    pub(crate) fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn calculate(matches: Vec<Match>) -> Self {
        let total = matches.len() as u32;
        if total == 0 {
            return PlayerScore::default();
        }
        let (mut kills, mut deaths, mut assists, mut wins, mut damage, mut duration) =
            (0, 0, 0, 0, 0, 0);
        let (mut prev_win, mut repeat) = (None, 1);
        matches.iter().for_each(|m| {
            kills += m.status.kills;
            deaths += m.status.deaths;
            assists += m.status.assists;
            wins += m.status.win as u32;
            damage += m.status.total_damage_dealt_to_champions;
            duration += m.game_duration;
            if prev_win.is_some() {
                if prev_win.unwrap() == m.status.win {
                    repeat += if m.status.win { 1 } else { -1 };
                } else {
                    repeat = if m.status.win { 1 } else { -1 };
                }
            }
            prev_win = Some(m.status.win);
        });
        PlayerScore {
            name: String::new(),
            kda: (kills + assists) as f32 / (deaths + 1) as f32,
            dpm: if duration > 0 {
                (damage as f32 / duration as f32) * 60.0 // 分均伤害
            } else {
                0.0
            },
            repeats: repeat,
            win_rate: wins as f32 / total as f32 * 100.0,
        }
    }
}

impl Display for PlayerScore {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let streak_text = if self.repeats > 0 { "连胜" } else { "连败" };
        write!(
            f,
            "{}战绩信息：\n\
            kda={:.2}，分均伤害={:.2}\n\
            胜率={:.2}%, {}{}",
            self.name,
            self.kda,
            self.dpm,
            self.win_rate,
            self.repeats.abs(),
            streak_text
        )
    }
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct Status {
    pub assists: u16,
    pub deaths: u16,
    pub kills: u16,
    pub win: bool,
    #[serde(rename = "totalDamageDealtToChampions")]
    pub total_damage_dealt_to_champions: u32,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Match {
    pub game_mode: String,
    // pub game_creation: u64, // 毫秒
    pub game_duration: u32, // 秒
    #[serde(
        deserialize_with = "deserialize_status_from_participants",
        rename = "participants"
    )]
    pub status: Status,
}

#[derive(Debug)]
pub(crate) struct Matches(pub Vec<Match>);

impl<'de> Deserialize<'de> for Matches {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
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

fn deserialize_status_from_participants<'de, D>(deserializer: D) -> Result<Status, D::Error>
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
