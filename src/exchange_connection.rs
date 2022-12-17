use serde::{Deserialize, Serialize};

pub mod orderbook {
    tonic::include_proto!("orderbook");
}
use orderbook::{Empty, Level, Summary};

type TokioWriteChannel = tokio::sync::mpsc::Sender<OrderbookUpdate>;
type TokioReceiveChannel = tokio::sync::mpsc::Receiver<OrderbookUpdate>;

#[derive(Deserialize, Serialize, Clone)]
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
    mut orders_a: Vec<OrderUpdate>,
    mut orders_b: Vec<OrderUpdate>,
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

/// A wrapper around Tokio channel with message building function
struct Sink {
    sink: TokioWriteChannel,
}
impl Sink {
    pub fn handle_update(
        &mut self,
        exchange: &str,
        asks: Vec<OrderUpdate>,
        bids: Vec<OrderUpdate>,
    ) -> () {
        println!("Sinked!: {} {}", asks.len(), bids.len())
    }
}
