use crate::error::{GeneralError, OrderbookResult};
use crate::exchange::exchange_client::OrderbookUpdate;
use crate::grpc::proto::{Level, Summary};
use itertools::Itertools;
use std::collections::HashMap;

type ExchangeName = String;
pub(crate) struct Merger {
    asks: HashMap<ExchangeName, Vec<Level>>,
    bids: HashMap<ExchangeName, Vec<Level>>,
}

impl Merger {
    pub(crate) fn new() -> Self {
        Self {
            asks: HashMap::new(),
            bids: HashMap::new(),
        }
    }

    pub(crate) fn update_exchange(
        &mut self,
        name: ExchangeName,
        orders: OrderbookUpdate,
    ) -> OrderbookResult<()> {
        self.asks.remove(&name);
        self.bids.remove(&name);
        let asks_size = std::cmp::max(10, orders.asks.len());
        let mut converted_asks = vec![];
        for i in 0..asks_size {
            converted_asks.push(Level::from_order(&name, orders.asks[i].clone())?);
        }
        let bids_size = std::cmp::max(10, orders.bids.len());
        let mut converted_bids = vec![];
        for i in 0..bids_size {
            converted_bids.push(Level::from_order(&name, orders.bids[i].clone())?);
        }
        self.bids.insert(name.clone(), converted_asks);
        self.asks.insert(name, converted_bids);
        Ok(())
    }

    pub(crate) fn provide_summary(&self) -> OrderbookResult<Summary> {
        let mut merged_asks = vec![];
        for (_, v) in self.asks.iter() {
            merged_asks = Self::merge_orders(true, &merged_asks, &v);
        }
        let mut merged_bids = vec![];
        for (_, v) in self.bids.iter() {
            merged_bids = Self::merge_orders(false, &merged_bids, &v);
        }
        let spread = merged_bids
            .get(0)
            .ok_or(GeneralError::orders_format_error())?
            .price
            - merged_asks
                .get(0)
                .ok_or(GeneralError::orders_format_error())?
                .price;

        Ok(Summary {
            spread,
            bids: merged_bids,
            asks: merged_asks,
        })
    }

    fn merge_orders(is_ask: bool, orders_a: &Vec<Level>, orders_b: &Vec<Level>) -> Vec<Level> {
        let a_iter = orders_a.iter().take(10);
        let b_iter = orders_b.iter().take(10);

        a_iter
            .merge_by(b_iter, |a, b| (a.price > b.price) == is_ask)
            .take(10)
            .cloned()
            .collect()
    }
}

#[test]
fn test_merge_orders() {
    use crate::exchange::exchange_client::OrderUpdate;
    let mut orders_a = vec![];
    let mut orders_b = vec![];
    for i in 0..10 {
        let mut order_a: OrderUpdate = OrderUpdate(vec![]);
        order_a.0.push((0.01f64 * i as f64).to_string());
        order_a.0.push((1f64 * i as f64).to_string());
        orders_a.push(crate::grpc::proto::Level::from_order("any", order_a));

        let mut order_b: OrderUpdate = OrderUpdate(vec![]);
        order_b.0.push((0.012f64 * i as f64).to_string());
        order_b.0.push((1.2f64 * i as f64).to_string());
        orders_b.push(crate::grpc::proto::Level::from_order("any", order_b));
    }

    let merged_orders = Merger::merge_orders(true, &orders_a, &orders_b);

    for i in merged_orders {
        println!("{:?}", i)
    }

    orders_a.reverse();
    orders_b.reverse();
    let merged_orders = Merger::merge_orders(false, &orders_a, &orders_b);
    for i in merged_orders {
        println!("{:?}", i)
    }
}
