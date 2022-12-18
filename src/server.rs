use futures_util::TryFutureExt;
use tokio::task::JoinHandle;

mod binance;
mod bitstamp;
mod client;
mod exchange_connection;
mod grpc;

struct Server;
use crate::binance::do_binance_v2;
use exchange_connection::{ExchangeClient, OrderbookUpdate};

impl Server {
    pub fn run_binance() -> tokio::sync::mpsc::Receiver<OrderbookUpdate> {
        let (write, read) = tokio::sync::mpsc::channel::<OrderbookUpdate>(4096);
        tokio::spawn(async move {
            do_binance_v2(write).await;
        });
        return read;
    }

    pub async fn run_server() {
        let mut binance_receiver = Self::run_binance();
        while let Some(v) = binance_receiver.recv().await {
            println!("GOT = {:?}", v);
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
