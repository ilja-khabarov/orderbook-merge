use crate::exchange::exchange_client::{WsReadChannel, WsWriteChannel};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{error, info};

use crate::error::{GeneralError, OrderbookResult};
use crate::exchange::exchange_client::{
    ExchangeClient2, OrderUpdate, OrderbookUpdate, TradingPair,
};

const BITSTAMP_ADDRESS: &str = "wss://ws.bitstamp.net";

/// The `channel` type for `data` field of Bitstamp subscribe request.
#[derive(Serialize, Deserialize)]
pub(crate) struct Channel {
    channel: String,
}

impl Channel {
    pub(crate) fn from_pair(pair: TradingPair) -> Self {
        Self {
            channel: format!("order_book_{}{}", pair.first_currency, pair.second_currency),
        }
    }
}

/// The message structure to subscribe to Bitstamp events.
/// Example:
/// {
///     "event": "bts:subscribe",
///     "data": {
///         "channel": "order_book_ethbtc"
///     }
/// }
#[derive(Serialize, Deserialize)]
pub(crate) struct BitstampSubMessage {
    event: String,
    data: Channel,
}

impl Default for BitstampSubMessage {
    fn default() -> Self {
        Self {
            event: "bts:subscribe".to_string(),
            data: Channel {
                channel: "order_book_ethbtc".to_string(),
            },
        }
    }
}

impl BitstampSubMessage {
    pub(crate) fn subscribe_to_pair(pair: TradingPair) -> Self {
        Self {
            event: "bts:subscribe".to_string(),
            data: Channel::from_pair(pair),
        }
    }
}

impl Display for BitstampSubMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(&self).expect("Failed to serialize BitstampSubMessage")
        )
    }
}

/// The 'data' field of Bitstamp response.
#[derive(Deserialize, Serialize)]
struct BitstampResponseData {
    timestamp: String,
    microtimestamp: String,
    bids: Vec<OrderUpdate>,
    asks: Vec<OrderUpdate>,
}

/// General Bitstamp response.
#[derive(Deserialize, Serialize)]
struct BitstampResponse {
    data: BitstampResponseData,
    channel: String,
    event: String,
}

pub(crate) struct BitstampClient {
    bitstamp_read_channel: WsReadChannel,
    bitstamp_write_channel: WsWriteChannel,
    grpc_sink: Sender<OrderbookUpdate>,
}

#[async_trait::async_trait]
impl ExchangeClient2 for BitstampClient {
    fn get_name() -> &'static str {
        "bitstamp"
    }

    fn get_address() -> &'static str {
        BITSTAMP_ADDRESS
    }

    async fn subscribe(&mut self) -> OrderbookResult<()> {
        let pair = TradingPair::default();
        let message_text = BitstampSubMessage::subscribe_to_pair(pair).to_string();
        let msg: Message = Message::text(message_text);
        self.bitstamp_write_channel
            .send(msg.clone())
            .await
            .map_err(|_| {
                GeneralError::connection_error("Failed to send subscribe message".to_string())
            })?;
        info!("Subscribe sent");
        let response = self.bitstamp_read_channel.next().await;
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
        self.bitstamp_read_channel
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

    async fn init2(grpc_sink: Sender<OrderbookUpdate>) -> Self {
        Self::init(grpc_sink).await
    }
}

impl BitstampClient {
    pub(crate) async fn init(grpc_sink: Sender<OrderbookUpdate>) -> Self {
        let address = Self::get_address();
        let url = url::Url::parse(&address).expect(&format!("Failed to parse URL: {}", address));

        let (ws_stream, _) = tokio_tungstenite::connect_async(url)
            .await
            .expect("Failed to connect");
        info!("Connection successful");

        let (bitstamp_write_channel, bitstamp_read_channel) = ws_stream.split();
        Self {
            bitstamp_read_channel,
            bitstamp_write_channel,
            grpc_sink,
        }
    }
    fn handle_message(message: Message) -> OrderbookResult<OrderbookUpdate> {
        let data = message.into_data();
        serde_json::from_slice::<BitstampResponse>(&data)
            .map(|response| OrderbookUpdate {
                bids: response.data.bids,
                asks: response.data.asks,
            })
            .map_err(Into::into)
    }
}
