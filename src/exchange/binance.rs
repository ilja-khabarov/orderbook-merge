use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::error::OrderbookResult;
use crate::exchange::exchange_client::{
    ExchangeClientConfig, OrderUpdate, OrderbookUpdate, TradingPair,
};

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
        serde_json::from_slice::<BinanceOrderbookResponse>(&data)
            .map(|response| OrderbookUpdate {
                bids: response.bids,
                asks: response.asks,
            })
            .map_err(Into::into)
    }
}
