use serde::{Deserialize, Serialize};

pub mod orderbook {
    tonic::include_proto!("orderbook");
}
use orderbook::{Empty, Level, Summary};

type TokioWriteChannel = tokio::sync::mpsc::Sender<OrderbookUpdate>;
type TokioReceiveChannel = tokio::sync::mpsc::Receiver<OrderbookUpdate>;

#[derive(Deserialize, Serialize)]
struct OrderUpdate(Vec<String>);

#[derive(Deserialize, Serialize)]
struct OrderbookUpdate {
    pub lastUpdateId: u64,
    pub bids: Vec<OrderUpdate>,
    pub asks: Vec<OrderUpdate>,
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
) -> Vec<Level> {
    todo!()
}
impl From<OrderbookUpdate> for Summary {
    fn from(orderbook_update: OrderbookUpdate) -> Self {
        let bids = orderbook_update.bids;
        let bids = orderbook_update.asks;
        Summary {
            spread: 1f64,
            bids: vec![],
            asks: vec![],
        }
    }
}

/// A wrapper around Tokio channel with message building function
struct Sink {
    sink: TokioWriteChannel,
}
impl Sink {
    pub fn handle_update(&mut self, asks: Vec<OrderUpdate>, bids: Vec<OrderUpdate>) -> () {
        println!("Sinked!: {} {}", asks.len(), bids.len())
    }
}
