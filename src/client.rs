pub mod orderbook {
    tonic::include_proto!("orderbook");
}
use orderbook::orderbook_aggregator_client::OrderbookAggregatorClient;
use orderbook::Empty;

use tonic::transport::Channel;
use tonic::Request;

use std::error::Error;
async fn stream_connect(
    client: &mut OrderbookAggregatorClient<Channel>,
) -> Result<(), Box<dyn Error>> {
    let mut stream = client
        .book_summary(Request::new(Empty {}))
        .await?
        .into_inner();

    while let Some(data) = stream.message().await? {
        println!("{:?}", data);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut client = OrderbookAggregatorClient::connect("http://127.0.0.1:8080").await?;

    stream_connect(&mut client).await?;

    Ok(())
}
