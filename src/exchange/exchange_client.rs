use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use tracing::{error, info};

type WsWriteChannel = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WsReadChannel = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct OrderUpdate(pub Vec<String>);

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct OrderbookUpdate {
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

pub(crate) struct ExchangeClient {
    sink: Sender<OrderbookUpdate>,
}

impl ExchangeClient {
    pub(crate) fn init(sink: Sender<OrderbookUpdate>) -> Self {
        Self { sink }
    }
    pub(crate) async fn init_connectors(address: &str) -> (WsWriteChannel, WsReadChannel) {
        let url = url::Url::parse(&address).expect(&format!("Failed to parse URL: {}", address));

        let (ws_stream, _) = tokio_tungstenite::connect_async(url)
            .await
            .expect("Failed to connect");
        info!("Connection successful");

        ws_stream.split()
    }
    pub(crate) async fn subscribe(
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
                Err(_) => error!("Failed to parse a message from exchange. Skipping response"),
            }
        })
        .await;
    }
}
