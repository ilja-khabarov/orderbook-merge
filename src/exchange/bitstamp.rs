use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::error::OrderbookResult;
use crate::exchange::exchange_client::{
    ExchangeClientConfig, OrderUpdate, OrderbookUpdate, TradingPair,
};

const BITSTAMP_ADDRESS: &str = "wss://ws.bitstamp.net";
const BITSTAMP_SUBSCRIBE: &str = r#"{
    "event": "bts:subscribe",
    "data": {
        "channel": "order_book_ethbtc"
    }
    }
    "#;

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

#[test]
fn test_default() {
    let sub = BitstampSubMessage::subscribe_to_pair(TradingPair::default());
    let s = sub.to_string();
    let sd: BitstampSubMessage = serde_json::from_str(&BITSTAMP_SUBSCRIBE).unwrap();
    let sd = sd.to_string();
    assert_eq!(s, sd);
}

#[test]
fn test_ethbtc() {
    let pair = TradingPair::default();
    let sub = BitstampSubMessage::subscribe_to_pair(pair);
    let s = sub.to_string();
    println!("{s}");
}

#[derive(Deserialize, Serialize)]
struct BitstampResponseData {
    timestamp: String,
    microtimestamp: String,
    bids: Vec<OrderUpdate>,
    asks: Vec<OrderUpdate>,
}

#[derive(Deserialize, Serialize)]
struct BitstampResponse {
    data: BitstampResponseData,
    channel: String,
    event: String,
}

pub(crate) struct BitstampClientConfig;

impl ExchangeClientConfig for BitstampClientConfig {
    fn get_name() -> &'static str {
        "bitstamp"
    }

    fn get_address() -> &'static str {
        BITSTAMP_ADDRESS
    }
    fn get_subscription_message(pair: TradingPair) -> String {
        BitstampSubMessage::subscribe_to_pair(pair).to_string()
    }
    fn message_handler(message: Message) -> OrderbookResult<OrderbookUpdate> {
        let data = message.into_data();
        serde_json::from_slice::<BitstampResponse>(&data)
            .map(|response| OrderbookUpdate {
                bids: response.data.bids,
                asks: response.data.asks,
            })
            .map_err(Into::into)
    }
}
