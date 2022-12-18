use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::{Arc, RwLock};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use tracing::{error, info};

pub type WsWriteChannel = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type WsReadChannel = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub mod orderbook {
    tonic::include_proto!("orderbook");
}
use orderbook::{Empty, Level, Summary};

pub type TokioWriteChannel = tokio::sync::mpsc::Sender<OrderbookUpdate>;
pub type TokioReceiveChannel = tokio::sync::mpsc::Receiver<OrderbookUpdate>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OrderUpdate(Vec<String>);

#[derive(Debug, Deserialize, Serialize)]
pub struct OrderbookUpdate {
    pub bids: Vec<OrderUpdate>,
    pub asks: Vec<OrderUpdate>,
}

pub(crate) trait ExchangeClientConfig {
    fn get_name() -> &'static str;
    fn get_address() -> &'static str;
    fn get_subscription_message() -> &'static str;
    fn message_handler(message: Message) -> anyhow::Result<OrderbookUpdate>;
}

pub(crate) async fn run_exchange_client<T>(
    local_write_channel: Sender<OrderbookUpdate>,
) -> anyhow::Result<()>
where
    T: ExchangeClientConfig,
{
    let mut client = ExchangeClient::init(local_write_channel);
    let (mut ws_write, mut ws_read) = ExchangeClient::init_connectors(T::get_address()).await;
    ExchangeClient::subscribe(&mut ws_read, &mut ws_write, T::get_subscription_message()).await?;
    client.run(ws_read, T::message_handler).await;
    Ok(())
}
/*
message Empty {}
message Summary {
double spread = 1;
repeated Level bids = 2;
repeated Level asks = 3;
}

message Level {
string exchange = 1;
double price = 2;
double amount = 3;
}
 */

fn merge_orders(
    is_ask: bool,
    orders_a: Vec<OrderUpdate>,
    orders_b: Vec<OrderUpdate>,
) -> Vec<OrderUpdate> {
    let mut aidx = 0;
    let mut bidx = 0;
    let mut merged = vec![];

    for i in 0..10 {
        let a = orders_a.get(aidx).unwrap();
        let a_price = a.0.get(0).unwrap();
        let b = orders_b.get(bidx).unwrap();
        let b_price = b.0.get(0).unwrap();

        if (a_price < b_price) == is_ask {
            // clone is expensive, but so easy
            merged.push(orders_a.get(aidx).unwrap().clone());
            aidx = aidx + 1;
        } else {
            merged.push(orders_b.get(bidx).unwrap().clone());
            bidx = bidx + 1;
        }
    }
    merged
}

#[test]
fn test_merge_orders() {
    let mut orders_a = vec![];
    let mut orders_b = vec![];
    for i in 0..10 {
        let mut order_a: OrderUpdate = OrderUpdate(vec![]);
        order_a.0.push((0.01f64 * i as f64).to_string());
        order_a.0.push((1f64 * i as f64).to_string());
        orders_a.push(order_a);

        let mut order_b: OrderUpdate = OrderUpdate(vec![]);
        order_b.0.push((0.012f64 * i as f64).to_string());
        order_b.0.push((1.2f64 * i as f64).to_string());
        orders_b.push(order_b);
    }

    let merged_orders = merge_orders(true, orders_a.clone(), orders_b.clone());

    for i in merged_orders {
        println!("{:?}", i.0)
    }

    orders_a.reverse();
    orders_b.reverse();
    let merged_orders = merge_orders(false, orders_a, orders_b);
    for i in merged_orders {
        println!("{:?}", i.0)
    }
}

impl Level {
    fn from_order(exchange: &str, update: OrderUpdate) -> Self {
        let price = update.0.get(0).unwrap().parse().unwrap();
        let amount = update.0.get(1).unwrap().parse().unwrap();
        Level {
            exchange: exchange.to_string(),
            price,
            amount,
        }
    }
}

impl Summary {
    fn from_orderbook(exchange: &str, orderbook_update: OrderbookUpdate) -> Self {
        let bids = orderbook_update
            .bids
            .into_iter()
            .map(|update| Level::from_order(exchange, update))
            .collect();
        let asks = orderbook_update
            .asks
            .into_iter()
            .map(|update| Level::from_order(exchange, update))
            .collect();
        Summary {
            spread: 1f64,
            bids: bids,
            asks: asks,
        }
    }
}

pub struct ExchangeClient {
    sink: TokioWriteChannel,
}

impl ExchangeClient {
    pub fn init(sink: TokioWriteChannel) -> Self {
        Self { sink }
    }
    pub async fn init_connectors(address: &str) -> (WsWriteChannel, WsReadChannel) {
        let url = url::Url::parse(&address).expect(&format!("Failed to parse URL: {}", address));

        let (ws_stream, _) = tokio_tungstenite::connect_async(url)
            .await
            .expect("Failed to connect");
        info!("Connection successful");

        ws_stream.split()
    }
    pub async fn subscribe(
        read: &mut WsReadChannel,
        write: &mut WsWriteChannel,
        message_text: &str,
    ) -> anyhow::Result<()> {
        let msg: Message = Message::text(message_text);
        write.send(msg).await?;
        info!("Subscribe sent");
        let response = read.next().await;
        match response {
            Some(Ok(m)) => {
                info!(
                    "Subscribe response received: {}",
                    m.into_text()
                        .unwrap_or("Error parsing exchange's response".to_string())
                );
            }
            _ => {
                error!("Failed to receive response to subscription");
            }
        }
        Ok(())
    }
    pub(crate) async fn run<F>(&mut self, read: WsReadChannel, handler: F) -> ()
    where
        F: Fn(Message) -> anyhow::Result<OrderbookUpdate>,
    {
        read.for_each(|message| async {
            match message {
                Ok(m) => {
                    if let Ok(update_converted) = handler(m) {
                        self.sink.send(update_converted).await.ok();
                    }
                }
                Err(e) => error!("Received faulty message from exchange. Skipping response"),
            }
        })
        .await;
    }
}
