use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::exchange_connection::{ExchangeClientConfig, OrderUpdate, OrderbookUpdate};

const BITSTAMP_ADDRESS: &str = "wss://ws.bitstamp.net";
const BITSTAMP_SUBSCRIBE: &str = r#"{
    "event": "bts:subscribe",
    "data": {
        "channel": "order_book_ethbtc"
    }
    }
    "#;

pub(crate) struct BitstampClientConfig;

impl ExchangeClientConfig for BitstampClientConfig {
    fn get_name() -> &'static str {
        "bitstamp"
    }

    fn get_address() -> &'static str {
        BITSTAMP_ADDRESS
    }
    fn get_subscription_message() -> &'static str {
        BITSTAMP_SUBSCRIBE
    }
    fn message_handler(message: Message) -> OrderbookUpdate {
        let data = message.into_data();
        let update = serde_json::from_slice::<BitstampResponse>(&data);
        match update {
            Ok(u) => {
                let orderbook_update = OrderbookUpdate {
                    bids: u.data.bids,
                    asks: u.data.asks,
                };
                return orderbook_update;
            }
            Err(e) => println!("Not Ok: {:?}\n", e),
        };
        unreachable!()
    }
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
