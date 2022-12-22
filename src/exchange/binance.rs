use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::exchange::exchange_client::{ExchangeClientConfig, OrderUpdate, OrderbookUpdate};

const BINANCE_ADDR: &str = "wss://stream.binance.com:9443/ws";

const BINANCE_SUBSCRIBE: &str = r#"{
  "method": "SUBSCRIBE",
  "params": [
    "ethbtc@depth10"
  ],
  "id": 1
}"#;

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
    fn get_subscription_message() -> &'static str {
        BINANCE_SUBSCRIBE
    }

    fn message_handler(message: Message) -> anyhow::Result<OrderbookUpdate> {
        let data = message.into_data();
        serde_json::from_slice::<BinanceResponse>(&data)
            .map(|response| OrderbookUpdate {
                bids: response.bids,
                asks: response.asks,
            })
            .map_err(Into::into)
    }
}
