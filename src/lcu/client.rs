use std::sync::Arc;

use futures_util::{TryStreamExt, sink::SinkExt};
use reqwest_websocket;
use reqwest_websocket::{Message, RequestBuilderExt};

use super::event::SUBSCRIBED_EVENT;
use super::{LcuMeta, handler::EventHandler};
use crate::context::HelperContext;
use crate::errors::HelperError;
use log::info;

pub struct LcuClient {
    client: Arc<reqwest::Client>,
    meta: LcuMeta,
}

impl LcuClient {
    pub fn new() -> anyhow::Result<Self> {
        let client = Arc::new(
            reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap(),
        );
        let mut meta = LcuMeta::new()?;
        Ok(LcuClient { client, meta })
    }

    pub async fn start_listener(&self, ctx: Arc<HelperContext>) -> anyhow::Result<()> {
        let url = self
            .meta
            .host_url
            .as_ref()
            .ok_or(HelperError::ClientCMDLineFailed)?;

        let response = self
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

        let handler = Arc::new(EventHandler::new(url, self.client.clone()));
        {
            let handler = handler.clone();
            let ctx = ctx.clone();
            tokio::spawn(async move {
                handler.update_summoner_info(ctx).await;
            });
        }
        while let Some(message) = ws.try_next().await? {
            if let Message::Text(text) = message {
                let handler = handler.clone();
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    handler.handle_message(&text, ctx).await;
                });
            }
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_port_and_token() -> anyhow::Result<()> {
    let client = LcuClient::new()?;
    client
        .start_listener(Arc::new(HelperContext::default()))
        .await?;
    Ok(())
}
