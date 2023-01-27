use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::error::OrderbookResult;
use crate::exchange::exchange_client::{ExchangeClientConfig, OrderUpdate, OrderbookUpdate};

const BITSTAMP_ADDRESS: &str = "wss://ws.bitstamp.net";
const BITSTAMP_SUBSCRIBE: &str = r#"{
    "event": "bts:subscribe",
    "data": {
        "channel": "order_book_ethbtc"
    }
    }
    "#;

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
    fn get_subscription_message() -> &'static str {
        BITSTAMP_SUBSCRIBE
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
