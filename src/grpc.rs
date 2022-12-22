use futures::lock::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};
use tracing::info;

use crate::exchange::exchange_client::OrderUpdate;

pub mod proto {
    tonic::include_proto!("orderbook");
}
use proto::orderbook_aggregator_server::{OrderbookAggregator, OrderbookAggregatorServer};
use proto::{Empty, Level, Summary};

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
                if let Some(v) = m.recv().await {
                    tx.send(Ok(v)).await.ok();
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

pub async fn run_grpc(receiver: Receiver<Summary>) -> anyhow::Result<()> {
    let address = "0.0.0.0:8080".parse().unwrap();
    let voting_service = OrderbookService::init(receiver);

    Server::builder()
        .add_service(OrderbookAggregatorServer::new(voting_service))
        .serve(address)
        .await?;
    Ok(())
}
