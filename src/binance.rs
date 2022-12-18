use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::exchange_connection::{ExchangeClientConfig, OrderUpdate, OrderbookUpdate};

const BINANCE_ADDR: &str = "wss://stream.binance.com:9443/ws";

const BINANCE_SUBSCRIBE: &str = r#"{
  "method": "SUBSCRIBE",
  "params": [
    "ethbtc@depth10"
  ],
  "id": 1
}"#;

#[derive(Debug, Deserialize, Serialize)]
pub struct BinanceResponse {
    pub bids: Vec<OrderUpdate>,
    pub asks: Vec<OrderUpdate>,
}

pub(crate) struct BinanceClientConfig;

impl ExchangeClientConfig for BinanceClientConfig {
    fn get_name() -> &'static str {
        "binance"
    }

    fn get_address() -> &'static str {
        BINANCE_ADDR
    }
    fn get_subscription_message() -> &'static str {
        BINANCE_SUBSCRIBE
    }

    fn message_handler(message: Message) -> OrderbookUpdate {
        let data = message
            .into_text()
            .expect("Failed to convert Message to String");

        if let Ok(update) = serde_json::from_str::<BinanceResponse>(&data) {
            let converted_update = OrderbookUpdate {
                bids: update.bids,
                asks: update.asks,
            };
            return converted_update;
        } else {
            panic!("Unexpected response: {}", data)
        }
    }
}
