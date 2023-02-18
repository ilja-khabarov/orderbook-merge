use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use tracing::{error, info};

use crate::error::{GeneralError, OrderbookResult};

type WsWriteChannel = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WsReadChannel = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// Simple exchange-agnostic `ask` or `bid` level.
/// Consists of two elements: <price> and <amount>.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct OrderUpdate(pub Vec<String>);

/// Whole orderbook update. A pair of vectors of `OrderUpdate`.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct OrderbookUpdate {
    pub bids: Vec<OrderUpdate>,
    pub asks: Vec<OrderUpdate>,
}

/// A pair of currencies. Rate is calculated as `first_currency`/`second_currency`.
#[derive(Clone)]
pub(crate) struct TradingPair {
    pub(crate) first_currency: String,
    pub(crate) second_currency: String,
}

impl Default for TradingPair {
    fn default() -> Self {
        Self {
            first_currency: "eth".to_string(),
            second_currency: "btc".to_string(),
        }
    }
}

/// Trait to implement for any exchange connection.
pub(crate) trait ExchangeClientConfig {
    /// Exchange name. Convention is to use it in snake case. E.g. `binance`.
    fn get_name() -> &'static str;
    /// Exchange websocket address. E.g. `wss://stream.binance.com:9443/ws`
    fn get_address() -> &'static str;
    /// Message to send to the exchange to subscribe on orderbook update events.
    fn get_subscription_message(pair: TradingPair) -> String;
    /// A function to convert message from the exchange format to local format.
    fn message_handler(message: Message) -> OrderbookResult<OrderbookUpdate>;
}

/// Runs an asynchronous exchange client.
pub(crate) async fn run_exchange_client<T>(
    local_write_channel: Sender<OrderbookUpdate>,
    trading_pair: TradingPair,
) -> OrderbookResult<()>
where
    T: ExchangeClientConfig,
{
    let mut client = ExchangeClient::init(local_write_channel);
    let (mut ws_write, mut ws_read) = ExchangeClient::init_connectors(T::get_address()).await;
    ExchangeClient::subscribe(
        &mut ws_read,
        &mut ws_write,
        &T::get_subscription_message(trading_pair),
    )
    .await?;
    client.run(ws_read, T::message_handler).await;
    Ok(())
}

/// Generic implementation of an exchange client.
/// Contains a simple 'sink' that is a sender channel to connect to a grpc server.
pub(crate) struct ExchangeClient {
    sink: Sender<OrderbookUpdate>,
}

impl ExchangeClient {
    pub(crate) fn init(sink: Sender<OrderbookUpdate>) -> Self {
        Self { sink }
    }

    /// Creates connectors to send/recv data from an exchange.
    pub(crate) async fn init_connectors(address: &str) -> (WsWriteChannel, WsReadChannel) {
        let url = url::Url::parse(&address).expect(&format!("Failed to parse URL: {}", address));

        let (ws_stream, _) = tokio_tungstenite::connect_async(url)
            .await
            .expect("Failed to connect");
        info!("Connection successful");

        ws_stream.split()
    }

    /// Send a response message and receive response.
    /// Currently just dumps the response.
    /// todo: Process response.
    pub(crate) async fn subscribe(
        read: &mut WsReadChannel,
        write: &mut WsWriteChannel,
        message_text: &str,
    ) -> OrderbookResult<()> {
        let msg: Message = Message::text(message_text);
        write.send(msg.clone()).await.map_err(|_| {
            GeneralError::connection_error("Failed to send subscribe message".to_string())
        })?;
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

    /// The client runner.
    /// Get message from the subscription channel, process it and send it further to the sink.
    pub(crate) async fn run<F>(&mut self, read: WsReadChannel, handler: F) -> ()
    where
        F: Fn(Message) -> OrderbookResult<OrderbookUpdate>,
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
