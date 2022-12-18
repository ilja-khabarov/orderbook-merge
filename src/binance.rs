use futures_util::stream::{SplitSink, SplitStream};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};

use crate::exchange_connection::{ExchangeClient, OrderUpdate, OrderbookUpdate};
use core::str::FromStr;

pub type AsyncWriteChannel = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type AsyncReadChannel = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

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

struct BinanceData;
impl BinanceData {
    pub fn get_address() -> &'static str {
        BINANCE_ADDR
    }
    pub fn get_subscription_message() -> &'static str {
        BINANCE_SUBSCRIBE
    }
}
fn binance_handler(message: Message) -> OrderbookUpdate {
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
pub async fn do_binance(local_write_channel: Sender<OrderbookUpdate>) {
    let mut client = ExchangeClient::init(local_write_channel);
    let (mut ws_write, mut ws_read) =
        ExchangeClient::init_connectors(BinanceData::get_address()).await;
    ExchangeClient::subscribe(
        &mut ws_read,
        &mut ws_write,
        BinanceData::get_subscription_message(),
    )
    .await;
    client.run(ws_read, binance_handler).await;
}
