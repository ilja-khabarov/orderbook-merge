use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::error::OrderbookResult;
use crate::exchange::exchange_client::{
    ExchangeClientConfig, OrderUpdate, OrderbookUpdate, TradingPair,
};

const BINANCE_ADDR: &str = "wss://stream.binance.com:9443/ws";

const BINANCE_SUBSCRIBE: &str = r#"{
  "method": "SUBSCRIBE",
  "params": [
    "ethbtc@depth10"
  ],
  "id": 1
}"#;

#[test]
fn test_binance_msg() {
    let s = BinanceSubMessage::subscribe_to_pair(TradingPair::default());
    let m = serde_json::to_string_pretty(&s);
    print!("{}", m.unwrap());
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Params(Vec<String>);

impl Params {
    pub(crate) fn from_pair(pair: TradingPair) -> Self {
        let s = format!("{}{}@depth10", pair.first_currency, pair.second_currency);
        Self(vec![s])
    }
}

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
        write!(f, "{}", serde_json::to_string(&self).unwrap())
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct BinanceResponse {
    bids: Vec<OrderUpdate>,
    asks: Vec<OrderUpdate>,
}

pub(crate) struct BinanceClientConfig;

impl ExchangeClientConfig for BinanceClientConfig {
    fn get_name() -> &'static str {
        "binance"
    }

    fn get_address() -> &'static str {
        BINANCE_ADDR
    }
    fn get_subscription_message(pair: TradingPair) -> String {
        BinanceSubMessage::subscribe_to_pair(pair).to_string()
    }

    fn message_handler(message: Message) -> OrderbookResult<OrderbookUpdate> {
        let data = message.into_data();
        serde_json::from_slice::<BinanceResponse>(&data)
            .map(|response| OrderbookUpdate {
                bids: response.bids,
                asks: response.asks,
            })
            .map_err(Into::into)
    }
}
