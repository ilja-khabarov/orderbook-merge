use crate::data::{Orderbook, PriceLevel};
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

const BINANCE_ADDR: &str = "wss://stream.binance.com:9443/ws";
const BINANCE_ADDR_RAW: &str = "wss://stream.binance.com:9443";

const BINANCE_SUBSCRIBE: &str = r#"{
  "method": "SUBSCRIBE",
  "params": [
    "ethbtc@depth"
  ],
  "id": 1
}"#;

#[derive(Deserialize, Serialize)]
struct OrderUpdate(Vec<String>);

#[derive(Deserialize, Serialize)]
struct DepthUpdate {
    pub e: String,
    pub E: u128,
    pub s: String,
    pub U: u128,
    pub u: u128,
    pub b: Vec<OrderUpdate>,
    pub a: Vec<OrderUpdate>,
}
struct BinanceClient {
    asks: Arc<RwLock<Orderbook>>,
    bids: Arc<RwLock<Orderbook>>,
}

impl BinanceClient {
    pub fn new(
        shared_orderbook_asks: Arc<RwLock<Orderbook>>,
        shared_orderbook_bids: Arc<RwLock<Orderbook>>,
    ) -> Self {
        Self {
            asks: shared_orderbook_asks.clone(),
            bids: shared_orderbook_bids.clone(),
        }
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
                tokio::io::stdout()
                    .write_all("Sub response received".as_bytes())
                    .await
                    .unwrap();
                tokio::io::stdout().write_all(&m.into_data()).await.unwrap();
                tokio::io::stdout()
                    .write_all("\n".as_bytes())
                    .await
                    .unwrap();
            }
            _ => {
                tokio::io::stdout()
                    .write_all("BBB".as_bytes())
                    .await
                    .unwrap();
                tokio::io::stdout()
                    .write_all("\n".as_bytes())
                    .await
                    .unwrap();
            }
        }
    }

    pub async fn run(state: Arc<RwLock<BinanceClient>>, read: AsyncReadChannel) -> () {
        read.for_each(|message| async {
            state.write().unwrap().handle_message(message.unwrap());

            /*
            let data = message.unwrap().into_text().unwrap();
            let update = serde_json::from_str::<DepthUpdate>(&data);
            match update {
                Ok(_) => tokio::io::stdout()
                    .write_all("Ok\n".as_bytes())
                    .await
                    .unwrap(),
                Err(e) => tokio::io::stdout()
                    .write_all(format!("Not Ok: {:?}\n", e).as_bytes())
                    .await
                    .unwrap(),
            };
            tokio::io::stdout()
                .write_all(data.as_bytes())
                .await
                .unwrap();
            tokio::io::stdout()
                .write_all("\n".as_bytes())
                .await
                .unwrap();
             */
        })
        .await;
    }

    fn handle_message(&mut self, message: Message) -> () {
        let data = message
            .into_text()
            .expect("Failed to convert Message to String");
        let v = Value::from_str(&data).expect("Failed to convert serde");
        let event = v.get("e").expect("Failed to get event name");
        let event_name = event.to_string();

        match event_name.as_str() {
            "\"depthUpdate\"" => {
                println!("Response: DepthUpdate");
                let v = serde_json::from_value::<DepthUpdate>(v).unwrap();
                let mut asks = self.asks.write().unwrap();
                let mut bids = self.bids.write().unwrap();
                let asks_update = v.a;
                let bids_update = v.b;

                for i in asks_update {
                    let price_level = PriceLevel::from(i.0.get(0).unwrap());
                    let price_value = i.0.get(1).unwrap().parse().unwrap();
                    (*asks).update_level(price_level, price_value);
                }
                println!("Asks updated. Now {}", (*asks).get_amount());
                for i in bids_update {
                    let price_level = PriceLevel::from(i.0.get(0).unwrap());
                    let price_value = i.0.get(1).unwrap().parse().unwrap();
                    (*bids).update_level(price_level, price_value);
                }
                println!("Bids updated. Now {}", (*bids).get_amount());
            }
            "\"result\"" => {
                println!("Response: result")
            }
            response => {
                println!("Unexpected response: {}", response)
            }
        }
    }
}

pub type AsyncWriteChannel = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type AsyncReadChannel = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub async fn do_binance() {
    let asks = Orderbook::new().arc();
    let bids = Orderbook::new().arc();
    let client = BinanceClient::new(asks, bids).arc();
    let (mut write, mut read) = BinanceClient::init_connectors().await;
    BinanceClient::subscribe(&mut read, &mut write).await;
    BinanceClient::run(client, read).await;
    /*
    let connect_addr = BINANCE_ADDR;
    let url = url::Url::parse(&connect_addr).unwrap();

    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("Connection successful");

    let (mut write  , read) = ws_stream.split();

    let msg: Message = Message::text(BINANCE_SUBSCRIBE);
    write.send(msg).await.unwrap();
    println!("Subscribe sent");

    let ws_to_stdout = {
        read.for_each(|message| async {
            let data = message.unwrap().into_data();
            let update = serde_json::from_slice::<DepthUpdate>(&data);
            match update {
                Ok(_) =>
                    tokio::io::stdout().write_all("Ok\n".as_bytes()).await.unwrap(),
                Err(e) => {
                    tokio::io::stdout().write_all(format!("Not Ok: {:?}\n", e).as_bytes()).await.unwrap()
                }
            };
            tokio::io::stdout().write_all(&data).await.unwrap();
            tokio::io::stdout().write_all("\n".as_bytes()).await.unwrap();
        })
    };

    ws_to_stdout.await;

     */
}
