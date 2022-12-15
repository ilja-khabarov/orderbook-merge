use futures::Stream;
use std::{error::Error, io::ErrorKind, net::ToSocketAddrs, pin::Pin, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};

pub mod orderbook {
    tonic::include_proto!("orderbook");
}
use orderbook::orderbook_aggregator_server::{OrderbookAggregator, OrderbookAggregatorServer};
use orderbook::{Empty, Level, Summary};

#[derive(Debug, Default)]
pub struct OrderbookService {}

async fn generator() -> Summary {
    tokio::time::sleep(Duration::from_secs(1)).await;
    Summary {
        spread: 1f64,
        bids: vec![],
        asks: vec![],
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
        println!("ListFeatures = {:?}", request);

        let (tx, rx) = mpsc::channel(4);

        tokio::spawn(async move {
            loop {
                let s: Summary = generator().await;
                println!("Summary generated");
                tx.send(Ok(s)).await.unwrap();
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

pub async fn run_grpc() -> Result<(), Box<dyn std::error::Error>> {
    let address = "127.0.0.1:8080".parse().unwrap();
    let voting_service = OrderbookService::default();

    Server::builder()
        .add_service(OrderbookAggregatorServer::new(voting_service))
        .serve(address)
        .await?;
    Ok(())
}
