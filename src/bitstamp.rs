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

use core::str::FromStr;

use crate::exchange_connection::{ExchangeClient, OrderUpdate, OrderbookUpdate};
const BITSTAMP_ADDR: &str = "wss://ws.bitstamp.net";

const BITSTAMP_SUBSCRIBE: &str = r#"{
    "event": "bts:subscribe",
    "data": {
        "channel": "order_book_ethbtc"
    }
}
"#;

struct BitstampData;
impl BitstampData {
    fn get_address() -> &'static str {
        "wss://ws.bitstamp.net"
    }
    fn get_subscription_message() -> &'static str {
        r#"{
        "event": "bts:subscribe",
        "data": {
            "channel": "order_book_ethbtc"
        }
        }
        "#
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

struct Sink;
impl Sink {
    pub fn handle_update(&mut self, asks: Vec<OrderUpdate>, bids: Vec<OrderUpdate>) -> () {
        println!("Sinked!: {} {}", asks.len(), bids.len())
    }
}

pub struct BitstampClient {
    sink: Sink,
}

pub type AsyncWriteChannel = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type AsyncReadChannel = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

impl BitstampClient {
    pub fn new() -> Self {
        Self { sink: Sink {} }
    }
    pub fn arc(self) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(self))
    }

    pub async fn init_connectors() -> (AsyncWriteChannel, AsyncReadChannel) {
        let connect_addr = BITSTAMP_ADDR;
        let url = url::Url::parse(&connect_addr).unwrap();

        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
        println!("Connection successful");

        ws_stream.split()
    }

    pub async fn subscribe(read: &mut AsyncReadChannel, write: &mut AsyncWriteChannel) -> () {
        let msg: Message = Message::text(BITSTAMP_SUBSCRIBE);
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

    pub async fn run(state: Arc<RwLock<BitstampClient>>, read: AsyncReadChannel) -> () {
        read.for_each(|message| async {
            state.write().unwrap().handle_message(message.unwrap());
        })
        .await;
    }

    fn handle_message(&mut self, message: Message) -> () {
        let data = message
            .into_text()
            .expect("Failed to convert Message to String");

        if let Ok(update) = serde_json::from_str::<BitstampResponse>(&data) {
            println!("--- Data");
            let asks_update = update.data.asks;
            let bids_update = update.data.bids;

            self.sink.handle_update(asks_update, bids_update);
        } else {
            println!("Unknown response. Ignoring: {}", data);
        }
    }
}

pub async fn do_bitstamp_v2() {
    let client = BitstampClient::new().arc();
    let (mut write, mut read) = BitstampClient::init_connectors().await;
    BitstampClient::subscribe(&mut read, &mut write).await;
    BitstampClient::run(client, read).await;
}

use tokio::sync::mpsc::Sender;
pub async fn do_bitstamp_v3(local_write_channel: Sender<OrderbookUpdate>) {
    let mut client = ExchangeClient::init(local_write_channel);
    let (mut ws_write, mut ws_read) =
        ExchangeClient::init_connectors(BitstampData::get_address()).await;
    ExchangeClient::subscribe(
        &mut ws_read,
        &mut ws_write,
        BitstampData::get_subscription_message(),
    )
    .await;
    client.run(ws_read, bitstamp_handler).await;
}

fn bitstamp_handler(message: Message) -> OrderbookUpdate {
    let data = message.into_data();
    let update = serde_json::from_slice::<BitstampResponse>(&data);
    match update {
        Ok(u) => {
            let orderbook_update = OrderbookUpdate {
                lastUpdateId: 0,
                bids: u.data.bids,
                asks: u.data.asks,
            };
            return orderbook_update;
        }
        Err(e) => println!("Not Ok: {:?}\n", e),
    };
    unreachable!()
}

pub async fn do_bitstamp() {
    let connect_addr = BITSTAMP_ADDR;
    let url = url::Url::parse(&connect_addr).unwrap();

    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("Connection successful");

    let (mut write, read) = ws_stream.split();

    let msg: Message = Message::text(BITSTAMP_SUBSCRIBE);
    write.send(msg).await.unwrap();
    println!("Subscribe sent");

    let ws_to_stdout = {
        read.for_each(|message| async {
            let data = message.unwrap().into_data();
            tokio::io::stdout()
                .write_all(format!("{}\n", String::from_utf8(data.clone()).unwrap()).as_bytes())
                .await
                .unwrap();
            let update = serde_json::from_slice::<BitstampResponse>(&data);
            match update {
                Ok(u) => {
                    tokio::io::stdout()
                        .write_all("Ok\n".as_bytes())
                        .await
                        .unwrap();
                    if u.data.bids.len() != 100 || u.data.asks.len() != 100 {
                        tokio::io::stdout()
                            .write_all("NOT 100\n".as_bytes())
                            .await
                            .unwrap();
                    }
                }
                Err(e) => tokio::io::stdout()
                    .write_all(format!("Not Ok: {:?}\n", e).as_bytes())
                    .await
                    .unwrap(),
            };
            tokio::io::stdout().write_all(&data).await.unwrap();
            tokio::io::stdout()
                .write_all("\n".as_bytes())
                .await
                .unwrap();
        })
    };

    ws_to_stdout.await;
}
