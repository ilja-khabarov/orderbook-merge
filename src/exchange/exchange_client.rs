use futures_util::stream::{SplitSink, SplitStream};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};

use crate::error::OrderbookResult;

pub(crate) type WsWriteChannel = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub(crate) type WsReadChannel = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

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

#[async_trait::async_trait]
pub(crate) trait ExchangeClient {
    fn get_name() -> &'static str;
    fn get_address() -> &'static str;
    async fn subscribe(&mut self) -> OrderbookResult<()>;
    async fn run(self);
    async fn init(grpc_sink: Sender<OrderbookUpdate>) -> Self;
}
