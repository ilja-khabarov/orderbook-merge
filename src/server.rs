mod binance;
mod bitstamp;
mod client;
mod exchange_connection;
mod grpc;

use crate::binance::do_binance;
use crate::bitstamp::do_bitstamp_v3;
use exchange_connection::{ExchangeClient, OrderbookUpdate};

struct Server;
impl Server {
    pub fn run_binance() -> tokio::sync::mpsc::Receiver<OrderbookUpdate> {
        let (binance_write, binance_read) = tokio::sync::mpsc::channel::<OrderbookUpdate>(4096);
        tokio::spawn(async move {
            do_binance(binance_write).await;
        });
        return binance_read;
    }
    pub fn run_bitstamp() -> tokio::sync::mpsc::Receiver<OrderbookUpdate> {
        let (bitstamp_write, bitstamp_read) = tokio::sync::mpsc::channel::<OrderbookUpdate>(4096);
        tokio::spawn(async move {
            do_bitstamp_v3(bitstamp_write).await;
        });
        return bitstamp_read;
    }

    pub async fn run_server() {
        let mut binance_receiver = Self::run_binance();
        let mut bitstamp_receiver = Self::run_bitstamp();

        loop {
            tokio::select! {
                msg = binance_receiver.recv() => {
                    println!("A: {:?}", msg)
                }
                msg = bitstamp_receiver.recv() => {
                    println!("B: {:?}", msg)
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    Server::run_server().await;
}
