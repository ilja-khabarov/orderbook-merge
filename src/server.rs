use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, watch::Sender as MultiSender, Mutex};

use crate::exchange::binance::BinanceClient;
use crate::exchange::bitstamp::BitstampClient;
use crate::exchange::exchange_client::ExchangeClient;
use crate::exchange::exchange_client::OrderbookUpdate;
use crate::exchange::exchange_client::TradingPair;
use crate::grpc::proto::Summary;
use crate::merger::Merger;

const TOKIO_CHANNEL_BUFFER_SIZE: usize = 4096;

pub(crate) struct Server;
impl Server {
    fn run_client<T>() -> Receiver<OrderbookUpdate>
    where
        T: ExchangeClient + Send,
    {
        let (write, read) =
            tokio::sync::mpsc::channel::<OrderbookUpdate>(TOKIO_CHANNEL_BUFFER_SIZE);
        tokio::spawn(async move {
            let mut client = T::init(write).await;
            client.subscribe().await.expect("Failed to subscribe");
            client.run().await;
        });

        return read;
    }

    pub(crate) async fn run_server(sender: MultiSender<Summary>, _trading_pair: TradingPair) {
        let mut bitstamp_receiver = Self::run_client::<BitstampClient>();
        let mut binance_receiver = Self::run_client::<BinanceClient>();
        let merger = Arc::new(Mutex::new(Merger::new()));

        loop {
            tokio::select! {
                msg = binance_receiver.recv() => {
                    if let Some(msg) = msg {
                        let mut lock = merger.lock().await;
                        lock.update_exchange(BinanceClient::get_name().to_string(), msg).ok();
                        tracing::debug!("Got a response from Binance");
                        match lock.provide_summary() {
                            Ok(summary) => {
                                sender.send(summary).ok();
                            },
                            Err(e) => tracing::error!("{:?}", e),
                        }
                    }
                }
                msg = bitstamp_receiver.recv() => {
                    if let Some(msg) = msg {
                        let mut lock = merger.lock().await;
                        lock.update_exchange(BitstampClient::get_name().to_string(), msg).ok();
                        tracing::debug!("Got a response from Stamp");
                        match lock.provide_summary() {
                            Ok(summary) => {
                                sender.send(summary).ok();
                            },
                            Err(e) => tracing::error!("{:?}", e),
                        }
                    }
                }
            }
        }
    }
}
