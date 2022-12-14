use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

type TradedValue = f64;
// todo: some float-based type
pub type PriceLevel = String;

pub struct Orderbook {
    data: BTreeMap<PriceLevel, TradedValue>,
}

impl Orderbook {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    pub fn arc(self) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(self))
    }

    pub fn get_amount(&self) -> usize {
        self.data.len()
    }

    pub fn update_level(&mut self, level: PriceLevel, value: TradedValue) {
        if value == 0.0 {
            self.data.remove(&level);
        } else {
            self.data.insert(level, value);
        }
    }
}
