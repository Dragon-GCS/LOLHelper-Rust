use std::sync::Arc;

use log::info;

use crate::{context::HelperContext, lcu::LcuClient};

const ACCEPT_API: &str = "/lol-matchmaking/v1/ready-check/accept";

impl LcuClient {
    pub(crate) async fn auto_accept(&self, ctx: Arc<HelperContext>) {
        let delay = *ctx.auto_accepted_delay.read().unwrap();
        if delay >= 0 {
            info!("将在 {delay} 秒后自动接受对局。");
            tokio::time::sleep(tokio::time::Duration::from_secs(delay as u64)).await;
        }
        let _ = self.post(ACCEPT_API).await.map_err(|e| {
            log::error!("自动接受对局失败: {e}");
        });
        info!("对局已自动接受");
        ctx.accepted
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
