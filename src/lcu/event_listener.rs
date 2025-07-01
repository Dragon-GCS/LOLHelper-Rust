use std::sync::Arc;

use futures_util::{TryStreamExt, sink::SinkExt};
use reqwest_websocket::{Message, RequestBuilderExt};

use super::LcuClient;
#[cfg(not(debug_assertions))]
use super::event::SUBSCRIBED_EVENT;
use crate::context::HelperContext;

pub async fn start_event_listener(
    lcu: Arc<LcuClient>,
    ctx: Arc<HelperContext>,
) -> anyhow::Result<()> {
    let url = lcu.host_url();
    let response = lcu
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
            handler.update_summoner_info(ctx).await;
        });
    }
    while let Some(message) = ws.try_next().await? {
        if let Message::Text(text) = message {
            let handler = lcu.clone();
            let ctx = ctx.clone();
            // tokio::spawn(async move {
            handler.handle_message(&text, ctx).await;
            // });
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_listener() -> anyhow::Result<()> {
    let client = Arc::new(LcuClient::new()?);
    let task = tokio::spawn(start_event_listener(
        client.clone(),
        Arc::new(HelperContext::default()),
    ));
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    task.abort();
    Ok(())
}
