use log::info;

use crate::{CONTEXT, LcuClient};
use std::sync::atomic::Ordering;

const ACCEPT_API: &str = "/lol-matchmaking/v1/ready-check/accept";

impl LcuClient {
    pub async fn auto_accept(&self) {
        let delay = CONTEXT.auto_accepted_delay.load(Ordering::Relaxed);
        info!("将在 {delay} 秒后自动接受对局。");
        tokio::time::sleep(tokio::time::Duration::from_secs(delay as u64)).await;
        let _ = self.post(ACCEPT_API).await.map_err(|e| {
            log::error!("自动接受对局失败: {e}");
        });
        info!("对局已自动接受");
        CONTEXT
            .accepted
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
