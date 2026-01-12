use std::sync::{Arc, atomic::Ordering};
use std::time::Duration;

use futures_util::{TryStreamExt, sink::SinkExt};
use log::{error, info};

use reqwest_websocket::{CloseCode, Message, RequestBuilderExt};
use tokio::sync::RwLock;

#[cfg(not(debug_assertions))]
use super::event::SUBSCRIBED_EVENT;
use crate::context::HelperContext;
use crate::lcu::Result;
use crate::lcu::{LcuClient, default_client};
#[cfg(not(debug_assertions))]
use log::debug;

pub async fn start_event_listener(
    lcu: Arc<RwLock<LcuClient>>,
    ctx: Arc<HelperContext>,
    cancel_token: Arc<tokio_util::sync::CancellationToken>,
) -> Result<()> {
    lcu.write().await.meta.refresh()?;
    let port = lcu.read().await.meta.port;
    let token = lcu.read().await.meta.token.clone();
    let mut ws = default_client()
        .get(format!("wss://127.0.0.1:{}", port))
        .basic_auth("riot", Some(&token))
        .timeout(Duration::from_secs(3))
        .upgrade()
        .send()
        .await?
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

    {
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
        let ctx = ctx.clone();
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
            let mut auto_pick = ctx.auto_pick.write().unwrap();
            auto_pick.unselected = champions;
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
    ctx.listening.store(false, Ordering::Relaxed);
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
