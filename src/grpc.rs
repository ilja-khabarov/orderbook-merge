use futures::Stream;
use futures_util::stream::FuturesUnordered;
use std::borrow::Borrow;
use std::{error::Error, io::ErrorKind, net::ToSocketAddrs, pin::Pin, time::Duration};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status, Streaming};
use tracing::info;

pub mod orderbook {
    tonic::include_proto!("orderbook");
}
use crate::exchange_connection::OrderbookUpdateReceiveChannel;
use orderbook::orderbook_aggregator_server::{OrderbookAggregator, OrderbookAggregatorServer};
use orderbook::{Empty, Level, Summary};

/// Mock event producer
async fn generator() -> Summary {
    tokio::time::sleep(Duration::from_secs(1)).await;
    Summary {
        spread: 1f64,
        bids: vec![],
        asks: vec![],
    }
}

//use futures_util::StreamExt;
use crate::exchange_connection::{OrderUpdate, OrderbookUpdate};
use futures::lock::Mutex;
use futures_util::AsyncWriteExt;
use std::cell::RefCell;
use std::io::Read;
use std::sync::{Arc, RwLock};

impl Level {
    pub(crate) fn from_order(exchange: &str, update: OrderUpdate) -> Self {
        let price = update.0.get(0).unwrap().parse().unwrap();
        let amount = update.0.get(1).unwrap().parse().unwrap();
        Level {
            exchange: exchange.to_string(),
            price,
            amount,
        }
    }
}

impl Summary {
    pub(crate) fn from_orderbook(exchange: &str, orderbook_update: OrderbookUpdate) -> Self {
        let bids = orderbook_update
            .bids
            .into_iter()
            .map(|update| Level::from_order(exchange, update))
            .collect();
        let asks = orderbook_update
            .asks
            .into_iter()
            .map(|update| Level::from_order(exchange, update))
            .collect();
        Summary {
            spread: 1f64,
            bids: bids,
            asks: asks,
        }
    }
}

#[derive(Debug)]
pub struct OrderbookService {
    summary_stream: Arc<Mutex<Receiver<Summary>>>,
}

impl OrderbookService {
    pub(crate) fn init(summary_stream: Receiver<Summary>) -> Self {
        Self {
            summary_stream: Arc::new(Mutex::new(summary_stream)),
        }
    }
}

#[tonic::async_trait]
impl OrderbookAggregator for OrderbookService {
    //type ListFeaturesStream = ReceiverStream<Result<Feature, Status>>;
    type BookSummaryStream = ReceiverStream<Result<Summary, Status>>;

    async fn book_summary(
        &self,
        request: Request<Empty>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        info!("ListFeatures = {:?}", request);

        let (tx, rx) = mpsc::channel(4096);

        let m = self.summary_stream.clone();
        tokio::spawn(async move {
            let mut m = m.lock().await;
            loop {
                let v: Summary = m.recv().await.unwrap();
                //let v = generator().await;
                info!("Summary generated");
                tx.send(Ok(v)).await.unwrap();
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

pub async fn run_grpc(receiver: Receiver<Summary>) -> anyhow::Result<()> {
    let address = "127.0.0.1:8080".parse().unwrap();
    let voting_service = OrderbookService::init(receiver);

    Server::builder()
        .add_service(OrderbookAggregatorServer::new(voting_service))
        .serve(address)
        .await?;
    Ok(())
}
