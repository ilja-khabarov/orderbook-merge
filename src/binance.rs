use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{connect_async, MaybeTlsStream, tungstenite::protocol::Message, WebSocketStream};
use futures_util::{future, pin_mut, SinkExt, StreamExt, stream::{SplitStream, SplitSink}};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;

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
    e: String,
    E: u128,
    s: String,
    U: u128,
    u: u128,
    b: Vec<OrderUpdate>,
    a: Vec<OrderUpdate>,
}

struct BinanceClient {

}

impl BinanceClient {
    pub async fn init_connectors() -> (AsyncWriteChannel, AsyncReadChannel) {
        let connect_addr = BINANCE_ADDR;
        let url = url::Url::parse(&connect_addr).unwrap();

        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
        println!("Connection successful");

        ws_stream.split()
    }

    pub async fn subscribe(read: &mut AsyncReadChannel, write: &mut AsyncWriteChannel ) -> () {
        let msg: Message = Message::text(BINANCE_SUBSCRIBE);
        write.send(msg).await.unwrap();
        println!("Subscribe sent");
        let response = read.next().await;
        match response {
            Some(Ok(m)) => {
                tokio::io::stdout().write_all("Sub response received".as_bytes()).await.unwrap();
                tokio::io::stdout().write_all(&m.into_data()).await.unwrap();
                tokio::io::stdout().write_all("\n".as_bytes()).await.unwrap();
            }
            _ => {
                tokio::io::stdout().write_all("BBB".as_bytes()).await.unwrap();
                tokio::io::stdout().write_all("\n".as_bytes()).await.unwrap();
            }
        }
    }

    pub async fn run(read: AsyncReadChannel) -> () {

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
        }).await;
    }
}

pub type AsyncWriteChannel = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type AsyncReadChannel = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub async fn do_binance() {
    let (mut write, mut read) = BinanceClient::init_connectors().await;
    BinanceClient::subscribe(&mut read, &mut write).await;
    BinanceClient::run(read).await;
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

