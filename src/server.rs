mod exchange;
mod grpc;
mod merger;

use tracing::info;
use tracing_subscriber;

use exchange::exchange_client::OrderbookUpdate;

use crate::exchange::binance::BinanceClientConfig;
use crate::exchange::bitstamp::BitstampClientConfig;
use crate::exchange::exchange_client::ExchangeClientConfig;
use crate::grpc::run_grpc;

const TOKIO_CHANNEL_BUFFER_SIZE: usize = 4096;

use merger::Merger;
use std::sync::Arc;
use tokio::sync::Mutex;

struct Server;
impl Server {
    pub fn run_exchange_client<T>() -> tokio::sync::mpsc::Receiver<OrderbookUpdate>
    where
        T: ExchangeClientConfig,
    {
        let (write, read) =
            tokio::sync::mpsc::channel::<OrderbookUpdate>(TOKIO_CHANNEL_BUFFER_SIZE);
        tokio::spawn(async move {
            // I'm open to talk about this unwrap(). Or any other, actually.
            exchange::exchange_client::run_exchange_client::<T>(write)
                .await
                .unwrap();
        });
        return read;
    }

    pub async fn run_server(sender: Sender<Summary>) {
        let mut binance_receiver = Self::run_exchange_client::<BinanceClientConfig>();
        let mut bitstamp_receiver = Self::run_exchange_client::<BitstampClientConfig>();
        let merger = Arc::new(Mutex::new(Merger::new()));

        loop {
            tokio::select! {
                msg = binance_receiver.recv() => {
                    //info!("Binance: {:?}", msg);
                    let mut lock = merger.lock().await;
                    lock.update_exchange("binance".to_string(), msg.unwrap());
                    let summary = lock.provide_summary();
                    info!("Binance: {:?}", summary);
                    sender.send(summary).await;
                }
                msg = bitstamp_receiver.recv() => {
                    //info!("Stamp: {:?}", msg);
                    let mut lock = merger.lock().await;
                    lock.update_exchange("bitstamp".to_string(), msg.unwrap());
                    let summary = lock.provide_summary();
                    info!("Stamp: {:?}", summary);
                    sender.send(summary).await;
                }
            }
        }
    }
}

use grpc::proto::Summary;
use tokio::sync::mpsc::Sender;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let (sender, receiver) = tokio::sync::mpsc::channel(4096);

    let server_handle = tokio::spawn(async move {
        Server::run_server(sender).await;
    });
    let grpc_handle = tokio::spawn(async move {
        run_grpc(receiver).await.unwrap();
    });

    tokio::join!(server_handle, grpc_handle);
}
