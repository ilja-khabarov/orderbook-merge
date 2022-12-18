use core::str::FromStr;
use futures_util::{
    future, pin_mut,
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc::Sender,
};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

use crate::exchange_connection::{ExchangeClient, OrderUpdate, OrderbookUpdate};

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
                bids: u.data.bids,
                asks: u.data.asks,
            };
            return orderbook_update;
        }
        Err(e) => println!("Not Ok: {:?}\n", e),
    };
    unreachable!()
}
