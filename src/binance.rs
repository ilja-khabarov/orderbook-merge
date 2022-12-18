use futures_util::{
    future, pin_mut,
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

use crate::exchange_connection::ExchangeClient;
use crate::exchange_connection::{OrderUpdate, OrderbookUpdate};
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

/// Should be implemented as write channel
struct Sink {}
impl Sink {
    pub fn handle_update(&mut self, asks: Vec<OrderUpdate>, bids: Vec<OrderUpdate>) -> () {
        println!("Sinked!: {} {}", asks.len(), bids.len())
    }
}

struct BinanceClient {
    sink: Sink,
}

impl BinanceClient {
    pub fn new() -> Self {
        Self { sink: Sink {} }
    }
    pub fn arc(self) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(self))
    }

    pub async fn init_connectors() -> (AsyncWriteChannel, AsyncReadChannel) {
        let connect_addr = BINANCE_ADDR;
        let url = url::Url::parse(&connect_addr).unwrap();

        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
        println!("Connection successful");

        ws_stream.split()
    }

    pub async fn subscribe(read: &mut AsyncReadChannel, write: &mut AsyncWriteChannel) -> () {
        let msg: Message = Message::text(BINANCE_SUBSCRIBE);
        write.send(msg).await.unwrap();
        println!("Subscribe sent");
        let response = read.next().await;
        match response {
            Some(Ok(m)) => {
                tokio::io::stdout().write_all(&m.into_data()).await.unwrap();
                tokio::io::stdout()
                    .write_all("\n".as_bytes())
                    .await
                    .unwrap();
            }
            _ => {
                tokio::io::stdout()
                    .write_all("Failed to receive response to subscription\n".as_bytes())
                    .await
                    .unwrap();
            }
        }
    }

    pub async fn run(state: Arc<RwLock<BinanceClient>>, read: AsyncReadChannel) -> () {
        read.for_each(|message| async {
            state.write().unwrap().handle_message(message.unwrap());
        })
        .await;
    }

    fn handle_message(&mut self, message: Message) -> () {
        let data = message
            .into_text()
            .expect("Failed to convert Message to String");

        if let Ok(update) = serde_json::from_str::<OrderbookUpdate>(&data) {
            let asks_update = update.asks;
            let bids_update = update.bids;
            self.sink.handle_update(asks_update, bids_update);
        } else {
            println!("Unexpected response: {}", data)
        }
    }
}

pub async fn do_binance() {
    let client = BinanceClient::new().arc();
    let (mut write, mut read) = BinanceClient::init_connectors().await;
    BinanceClient::subscribe(&mut read, &mut write).await;
    BinanceClient::run(client, read).await;
}

use crate::exchange_connection::TokioWriteChannel;

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

    if let Ok(update) = serde_json::from_str::<OrderbookUpdate>(&data) {
        println!("Sinked!: {} {}", update.asks.len(), update.bids.len());
        return update;
    } else {
        panic!("Unexpected response: {}", data)
    }
}
pub async fn do_binance_v2() {
    let (write, read) = tokio::sync::mpsc::channel::<OrderbookUpdate>(4096);
    let mut client = ExchangeClient::init(write);
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
