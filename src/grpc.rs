use futures::lock::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc::{self};
use tokio::sync::watch::Receiver as MultiReceiver;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};
use tracing::info;

use crate::exchange::exchange_client::OrderUpdate;

pub mod proto {
    tonic::include_proto!("orderbook");
}
use proto::orderbook_aggregator_server::{OrderbookAggregator, OrderbookAggregatorServer};
use proto::{Empty, Level, Summary};

use crate::error::{GeneralError, OrderbookResult};
impl Level {
    pub(crate) fn from_order(exchange: &str, update: OrderUpdate) -> OrderbookResult<Self> {
        let price = update
            .0
            .get(0)
            .ok_or(GeneralError::orders_format_error())?
            .parse()?;
        let amount = update
            .0
            .get(1)
            .ok_or(GeneralError::orders_format_error())?
            .parse()?;
        Ok(Level {
            exchange: exchange.to_string(),
            price,
            amount,
        })
    }
}

use std::cmp::Ordering;

impl PartialOrd for Level {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.price - other.price > 1e-10 {
            return Some(Ordering::Greater);
        } else if other.price - self.price > 1e-10 {
            return Some(Ordering::Less);
        }
        Some(Ordering::Equal)
    }
}

#[derive(Debug)]
pub struct OrderbookService {
    summary_stream: Arc<Mutex<MultiReceiver<Summary>>>,
}

impl OrderbookService {
    pub(crate) fn init(summary_stream: MultiReceiver<Summary>) -> Self {
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
            let mut local_receiver = m.lock().await.clone();
            loop {
                if let Ok(()) = local_receiver.changed().await {
                    let summary: Summary = local_receiver.borrow().clone();
                    if let Err(_) = tx.send(Ok(summary)).await {
                        info!("Client connection closed");
                        break;
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

pub async fn run_grpc(receiver: MultiReceiver<Summary>) -> Result<(), Box<dyn std::error::Error>> {
    let address = "0.0.0.0:8080".parse()?;
    let service = OrderbookService::init(receiver);

    Server::builder()
        .add_service(OrderbookAggregatorServer::new(service))
        .serve(address)
        .await?;
    Ok(())
}
