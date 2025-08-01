use std::{sync::Arc, time::Duration};

use futures_util::{TryStreamExt, sink::SinkExt};
use log::{error, info};

use anyhow::anyhow;
use reqwest_websocket::{CloseCode, Message, RequestBuilderExt};
use tokio::sync::RwLock;

use super::LcuClient;
#[cfg(not(debug_assertions))]
use super::event::SUBSCRIBED_EVENT;
use crate::context::HelperContext;
#[cfg(not(debug_assertions))]
use log::debug;

pub async fn start_event_listener(
    lcu: Arc<RwLock<LcuClient>>,
    ctx: Arc<HelperContext>,
    cancel_token: Arc<tokio_util::sync::CancellationToken>,
) -> anyhow::Result<()> {
    let url = { lcu.write().await.meta.refresh()? };
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let mut ws = client
        .get(format!("wss://{}", url))
        .timeout(Duration::from_secs(3))
        .upgrade()
        .send()
        .await
        .map_err(|_| anyhow!("客户端连接失败，请检查代理是否关闭"))?
        .into_websocket()
        .await?;

    #[cfg(debug_assertions)]
    ws.send(Message::Text("[5, \"OnJsonApiEvent\"]".into()))
        .await?;
    #[cfg(not(debug_assertions))]
    for event in SUBSCRIBED_EVENT {
        debug!("subscribed event: {event}");
        ws.send(Message::Text(format!("[5, \"OnJsonApiEvent_{event}\"]")))
            .await?;
    }

    if ctx.clone().me.read().unwrap().game_name.is_empty() {
        let lcu = lcu.clone();
        let ctx = ctx.clone();
        tokio::spawn(async move {
            lcu.read()
                .await
                .update_summoner_info(ctx)
                .await
                .unwrap_or_else(|e| {
                    error!("更新玩家信息失败: {e}");
                });
        });
    }
    if ctx.auto_pick.read().unwrap().unselected.is_empty() {
        let lcu = lcu.clone();
        let ctx1 = ctx.clone();
        tokio::spawn(async move {
            let champions = lcu
                .read()
                .await
                .get_owned_champions()
                .await
                .unwrap_or_else(|e| {
                    error!("加载自动选择数据失败: {e}");
                    vec![]
                });
            let mut auto_pick = ctx1.auto_pick.write().unwrap();
            auto_pick.unselected = champions;
            auto_pick.save();
        });
    }
    info!("客户端监听已启动");
    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                break;
            }
            Ok(Some(message)) = ws.try_next() => {
                let handler = lcu.clone();
                let ctx = ctx.clone();
                if let Message::Text(text) = message {
                    // tokio::spawn(async move {
                    handler
                        .read()
                        .await
                        .handle_message(text, ctx)
                        .await
                        .unwrap_or_else(|e| {
                            error!("处理消息失败: {e}");
                        });
                    // });
                }
            }
            else => {
                break;
            }
        }
    }

    ws.close(CloseCode::Normal, None).await?;
    *ctx.listening.write().unwrap() = false;
    info!("客户端监听已停止");
    Ok(())
}

#[tokio::test]
async fn test_listener() -> anyhow::Result<()> {
    let client = Arc::new(RwLock::new(LcuClient::default()));
    let ctx = Arc::new(HelperContext::new());
    let cancel_token = Arc::new(tokio_util::sync::CancellationToken::new());
    let task = tokio::spawn(start_event_listener(client, ctx, cancel_token.clone()));

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    cancel_token.cancel();
    task.await??;
    Ok(())
}
