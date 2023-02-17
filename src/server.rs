use std::sync::Arc;
use tokio::sync::{mpsc::Receiver, watch::Sender as MultiSender, Mutex};

use crate::exchange::exchange_client::TradingPair;
use crate::exchange::{
    binance::BinanceClientConfig,
    bitstamp::BitstampClientConfig,
    exchange_client::{ExchangeClientConfig, OrderbookUpdate},
};
use crate::grpc::proto::Summary;
use crate::merger::Merger;

const TOKIO_CHANNEL_BUFFER_SIZE: usize = 4096;

pub(crate) struct Server;
impl Server {
    fn run_exchange_client<T>(trading_pair: TradingPair) -> Receiver<OrderbookUpdate>
    where
        T: ExchangeClientConfig,
    {
        let (write, read) =
            tokio::sync::mpsc::channel::<OrderbookUpdate>(TOKIO_CHANNEL_BUFFER_SIZE);
        tokio::spawn(async move {
            // I'm open to talk about this unwrap(). Or any other, actually.
            crate::exchange::exchange_client::run_exchange_client::<T>(write, trading_pair)
                .await
                .unwrap();
        });
        return read;
    }

    pub(crate) async fn run_server(sender: MultiSender<Summary>, trading_pair: TradingPair) {
        let mut binance_receiver =
            Self::run_exchange_client::<BinanceClientConfig>(trading_pair.clone());
        let mut bitstamp_receiver = Self::run_exchange_client::<BitstampClientConfig>(trading_pair);
        let merger = Arc::new(Mutex::new(Merger::new()));

        loop {
            tokio::select! {
                msg = binance_receiver.recv() => {
                    if let Some(msg) = msg {
                        let mut lock = merger.lock().await;
                        lock.update_exchange(BinanceClientConfig::get_name().to_string(), msg).ok();
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
                        lock.update_exchange(BitstampClientConfig::get_name().to_string(), msg).ok();
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
