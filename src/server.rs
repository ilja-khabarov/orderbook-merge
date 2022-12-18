mod binance;
mod bitstamp;
mod client;
mod exchange_connection;
mod grpc;
use tracing::info;
use tracing_subscriber;

use exchange_connection::OrderbookUpdate;

use crate::binance::BinanceClientConfig;
use crate::bitstamp::BitstampClientConfig;
use crate::exchange_connection::ExchangeClientConfig;

const TOKIO_CHANNEL_BUFFER_SIZE: usize = 4096;

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
            exchange_connection::run_exchange_client::<T>(write)
                .await
                .unwrap();
        });
        return read;
    }

    pub async fn run_server() {
        let mut binance_receiver = Self::run_exchange_client::<BinanceClientConfig>();
        let mut bitstamp_receiver = Self::run_exchange_client::<BitstampClientConfig>();

        loop {
            tokio::select! {
                msg = binance_receiver.recv() => {
                    info!("A: {:?}", msg)
                }
                msg = bitstamp_receiver.recv() => {
                    info!("B: {:?}", msg)
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    Server::run_server().await;
}
