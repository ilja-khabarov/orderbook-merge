use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::{Arc, RwLock};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use tracing::{error, info};

pub type WsWriteChannel = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type WsReadChannel = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

use crate::client::orderbook::{Empty, Level, Summary};

pub type OrderbookUpdateSendChannel = tokio::sync::mpsc::Sender<OrderbookUpdate>;
pub type OrderbookUpdateReceiveChannel = tokio::sync::mpsc::Receiver<OrderbookUpdate>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OrderUpdate(pub Vec<String>);

#[derive(Debug, Deserialize, Serialize)]
pub struct OrderbookUpdate {
    pub bids: Vec<OrderUpdate>,
    pub asks: Vec<OrderUpdate>,
}

pub(crate) trait ExchangeClientConfig {
    fn get_name() -> &'static str;
    fn get_address() -> &'static str;
    fn get_subscription_message() -> &'static str;
    fn message_handler(message: Message) -> anyhow::Result<OrderbookUpdate>;
}

pub(crate) async fn run_exchange_client<T>(
    local_write_channel: Sender<OrderbookUpdate>,
) -> anyhow::Result<()>
where
    T: ExchangeClientConfig,
{
    let mut client = ExchangeClient::init(local_write_channel);
    let (mut ws_write, mut ws_read) = ExchangeClient::init_connectors(T::get_address()).await;
    ExchangeClient::subscribe(&mut ws_read, &mut ws_write, T::get_subscription_message()).await?;
    client.run(ws_read, T::message_handler).await;
    Ok(())
}

pub struct ExchangeClient {
    sink: OrderbookUpdateSendChannel,
}

impl ExchangeClient {
    pub fn init(sink: OrderbookUpdateSendChannel) -> Self {
        Self { sink }
    }
    pub async fn init_connectors(address: &str) -> (WsWriteChannel, WsReadChannel) {
        let url = url::Url::parse(&address).expect(&format!("Failed to parse URL: {}", address));

        let (ws_stream, _) = tokio_tungstenite::connect_async(url)
            .await
            .expect("Failed to connect");
        info!("Connection successful");

        ws_stream.split()
    }
    pub async fn subscribe(
        read: &mut WsReadChannel,
        write: &mut WsWriteChannel,
        message_text: &str,
    ) -> anyhow::Result<()> {
        let msg: Message = Message::text(message_text);
        write.send(msg).await?;
        info!("Subscribe sent");
        let response = read.next().await;
        match response {
            Some(Ok(m)) => {
                info!(
                    "Subscribe response received: {}",
                    m.into_text()
                        .unwrap_or("Error parsing exchange's response".to_string())
                );
            }
            _ => {
                error!("Failed to receive response to subscription");
            }
        }
        Ok(())
    }
    pub(crate) async fn run<F>(&mut self, read: WsReadChannel, handler: F) -> ()
    where
        F: Fn(Message) -> anyhow::Result<OrderbookUpdate>,
    {
        read.for_each(|message| async {
            match message {
                Ok(m) => {
                    if let Ok(update_converted) = handler(m) {
                        self.sink.send(update_converted).await.ok();
                    }
                }
                Err(e) => error!("Received faulty message from exchange. Skipping response"),
            }
        })
        .await;
    }
}
