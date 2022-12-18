use futures_util::TryFutureExt;
use tokio::task::JoinHandle;

mod binance;
mod bitstamp;
mod client;
mod exchange_connection;
mod grpc;

struct Server;
use crate::binance::do_binance_v2;
use crate::bitstamp::do_bitstamp_v3;
use exchange_connection::{ExchangeClient, OrderbookUpdate};

impl Server {
    pub fn run_binance() -> tokio::sync::mpsc::Receiver<OrderbookUpdate> {
        let (binance_write, binance_read) = tokio::sync::mpsc::channel::<OrderbookUpdate>(4096);
        tokio::spawn(async move {
            do_binance_v2(binance_write).await;
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
                    println!("AAA")
                }
                msg = bitstamp_receiver.recv() => {
                    println!("--- BBB")
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    Server::run_server().await;
    std::thread::sleep(std::time::Duration::from_secs(5));
    //binance::do_binance_v2().await;
    //bitstamp::do_bitstamp_v3().await;
    //binance::do_binance().await;
    //grpc::run_grpc().await.unwrap();
    //bitstamp::do_bitstamp_v2().await;
}
