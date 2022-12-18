mod binance;
mod bitstamp;
mod client;
mod exchange_connection;
mod grpc;

use exchange_connection::{ExchangeClient, OrderbookUpdate};

use crate::binance::BinanceClientConfig;
use crate::bitstamp::BitstampClientConfig;
use crate::exchange_connection::ExchangeClientConfig;

struct Server;
impl Server {
    pub fn run_exchange_client<T>() -> tokio::sync::mpsc::Receiver<OrderbookUpdate>
    where
        T: ExchangeClientConfig,
    {
        let (write, read) = tokio::sync::mpsc::channel::<OrderbookUpdate>(4096);
        tokio::spawn(async move {
            exchange_connection::do_any::<T>(write).await;
        });
        return read;
    }

    pub async fn run_server() {
        let mut binance_receiver = Self::run_exchange_client::<BinanceClientConfig>();
        let mut bitstamp_receiver = Self::run_exchange_client::<BitstampClientConfig>();

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
