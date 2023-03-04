use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::{
    error::GeneralError,
    error::OrderbookResult,
    exchange::exchange_client::{ExchangeClient, WsReadChannel, WsWriteChannel},
    exchange::exchange_client::{OrderUpdate, OrderbookUpdate, TradingPair},
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::Sender;
use tracing::{error, info};

const BINANCE_ADDR: &str = "wss://stream.binance.com:9443/ws";

/// The `params` field of Binance subscribe request.
/// Example: "ethbtc@depth10", where `etcbtc` is trading pair, and `depth` is amount of top
/// asks/bids in pair.
#[derive(Deserialize, Serialize)]
pub(crate) struct Params(Vec<String>);

impl Params {
    pub(crate) fn from_pair(pair: TradingPair) -> Self {
        let s = format!("{}{}@depth10", pair.first_currency, pair.second_currency);
        Self(vec![s])
    }
}

/// The message structure to subscribe to Binance events.
/// Example:
/// {
///  "method": "SUBSCRIBE",
///  "params": [
///     "ethbtc@depth10"
///   ],
///   "id": 1
/// }
#[derive(Deserialize, Serialize)]
struct BinanceSubMessage {
    method: String,
    params: Params,
    id: u32,
}

impl BinanceSubMessage {
    pub(crate) fn subscribe_to_pair(pair: TradingPair) -> Self {
        Self {
            method: "SUBSCRIBE".to_string(),
            params: Params::from_pair(pair),
            id: 1,
        }
    }
}

impl Display for BinanceSubMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(&self).map_err(|_| std::fmt::Error)?
        )
    }
}

/// Typical stream data message from Binance.
#[derive(Debug, Deserialize, Serialize)]
struct BinanceOrderbookResponse {
    bids: Vec<OrderUpdate>,
    asks: Vec<OrderUpdate>,
}

pub(crate) struct BinanceClient {
    binance_read_channel: WsReadChannel,
    binance_write_channel: WsWriteChannel,
    grpc_sink: Sender<OrderbookUpdate>,
}

#[async_trait::async_trait]
impl ExchangeClient for BinanceClient {
    fn get_name() -> &'static str {
        "binance"
    }

    fn get_address() -> &'static str {
        BINANCE_ADDR
    }

    async fn subscribe(&mut self) -> OrderbookResult<()> {
        let pair = TradingPair::default();
        let message_text = BinanceSubMessage::subscribe_to_pair(pair).to_string();
        let msg: Message = Message::text(message_text);
        self.binance_write_channel
            .send(msg.clone())
            .await
            .map_err(|_| {
                GeneralError::connection_error("Failed to send subscribe message".to_string())
            })?;
        info!("Subscribe sent");
        let response = self.binance_read_channel.next().await;
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

    async fn run(self) {
        self.binance_read_channel
            .for_each(|message| async {
                match message {
                    Ok(m) => {
                        if let Ok(update_converted) = Self::handle_message(m) {
                            self.grpc_sink.send(update_converted).await.ok();
                        }
                    }
                    Err(_) => error!("Failed to parse a message from exchange. Skipping response"),
                }
            })
            .await;
    }

    async fn init(grpc_sink: Sender<OrderbookUpdate>) -> Self {
        Self::init(grpc_sink).await
    }
}

impl BinanceClient {
    pub(crate) async fn init(grpc_sink: Sender<OrderbookUpdate>) -> Self {
        let address = Self::get_address();
        let url = url::Url::parse(&address).expect(&format!("Failed to parse URL: {}", address));

        let (ws_stream, _) = tokio_tungstenite::connect_async(url)
            .await
            .expect("Failed to connect");
        info!("Connection successful");

        let (binance_write_channel, binance_read_channel) = ws_stream.split();
        Self {
            binance_read_channel,
            binance_write_channel,
            grpc_sink,
        }
    }
    fn handle_message(message: Message) -> OrderbookResult<OrderbookUpdate> {
        let data = message.into_data();
        serde_json::from_slice::<BinanceOrderbookResponse>(&data)
            .map(|response| OrderbookUpdate {
                bids: response.bids,
                asks: response.asks,
            })
            .map_err(Into::into)
    }
}
