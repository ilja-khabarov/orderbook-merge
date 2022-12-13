use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{future, pin_mut, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};



#[derive(Deserialize, Serialize)]
struct OrderUpdate(Vec<String>);

const BITSTAMP_ADDR: &str = "wss://ws.bitstamp.net";

const BITSTAMP_SUBSCRIBE: &str = r#"{
    "event": "bts:subscribe",
    "data": {
        "channel": "order_book_ethbtc"
    }
}
"#;


#[derive(Deserialize, Serialize)]
struct BitstampData {
    timestamp: String,
    microtimestamp: String,
    bids: Vec<OrderUpdate>,
    asks: Vec<OrderUpdate>,
}

#[derive(Deserialize, Serialize)]
struct BitstampResponse {
    data: BitstampData,
    channel: String,
    event: String,
}

pub async fn do_bitstamp() {
    let connect_addr = BITSTAMP_ADDR;
    let url = url::Url::parse(&connect_addr).unwrap();

    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("Connection successful");

    let (mut write  , read) = ws_stream.split();

    let msg: Message = Message::text(BITSTAMP_SUBSCRIBE);
    write.send(msg).await.unwrap();
    println!("Subscribe sent");

    let ws_to_stdout = {
        read.for_each(|message| async {
            let data = message.unwrap().into_data();
            let update = serde_json::from_slice::<BitstampResponse>(&data);
            match update {
                Ok(u) => {
                    tokio::io::stdout().write_all("Ok\n".as_bytes()).await.unwrap();
                    if u.data.bids.len() != 100 || u.data.asks.len() != 100 {
                        tokio::io::stdout().write_all("NOT 100\n".as_bytes()).await.unwrap();
                    }
                }
                Err(e) => {
                    tokio::io::stdout().write_all(format!("Not Ok: {:?}\n", e).as_bytes()).await.unwrap()
                }
            };
            tokio::io::stdout().write_all(&data).await.unwrap();
            tokio::io::stdout().write_all("\n".as_bytes()).await.unwrap();
        })
    };

    ws_to_stdout.await;

}
