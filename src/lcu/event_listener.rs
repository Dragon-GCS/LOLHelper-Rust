use std::sync::Arc;

use futures_util::{TryStreamExt, sink::SinkExt};
use log::{error, info};
use reqwest_websocket::{Message, RequestBuilderExt};
use tokio::sync::RwLock;

use super::LcuClient;
#[cfg(not(debug_assertions))]
use super::event::SUBSCRIBED_EVENT;
use crate::context::HelperContext;

pub async fn start_event_listener(
    lcu: Arc<RwLock<LcuClient>>,
    ctx: Arc<HelperContext>,
) -> anyhow::Result<()> {
    let lcu_guard = lcu.read().await;
    let url = lcu_guard.host_url();
    let response = lcu_guard
        .client
        .get(format!("wss://{url}"))
        .upgrade()
        .send()
        .await?;

    let mut ws = response.into_websocket().await?;
    #[cfg(debug_assertions)]
    ws.send(Message::Text("[5, \"OnJsonApiEvent\"]".into()))
        .await?;
    #[cfg(not(debug_assertions))]
    for event in SUBSCRIBED_EVENT {
        info!("subscribed event: {event}");
        ws.send(Message::Text(format!("[5, \"OnJsonApiEvent_{event}\"]")))
            .await?;
    }

    {
        let handler = lcu.clone();
        let ctx = ctx.clone();
        tokio::spawn(async move {
            handler
                .read()
                .await
                .update_summoner_info(ctx)
                .await
                .unwrap_or_else(|e| {
                    error!("更新玩家信息失败: {e}");
                });
        });
    }
    while *ctx.listening.read().unwrap() {
        let Some(message) = ws.try_next().await? else {
            break;
        };

        if let Message::Text(text) = message {
            let handler = lcu.clone();
            let ctx = ctx.clone();
            // tokio::spawn(async move {
            handler
                .read()
                .await
                .handle_message(&text, ctx)
                .await
                .unwrap_or_else(|e| {
                    error!("处理消息失败: {e}");
                });
            // });
        }
    }
    info!("Event listener stopped.");
    Ok(())
}

#[tokio::test]
async fn test_listener() -> anyhow::Result<()> {
    let mut client = LcuClient::default();
    client.meta.refresh()?;
    let task = tokio::spawn(start_event_listener(
        Arc::new(RwLock::new(client)),
        Arc::new(HelperContext::new()),
    ));
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    task.abort();
    Ok(())
}
