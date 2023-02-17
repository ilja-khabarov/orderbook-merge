mod error;
mod exchange;
mod grpc;
mod merger;
mod server;

use tracing_subscriber;

use crate::exchange::exchange_client::TradingPair;
use crate::grpc::proto::Summary;
use crate::grpc::run_grpc;
use server::Server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let trading_pair = TradingPair::default();

    let (sender, receiver) = tokio::sync::watch::channel(Summary::default());

    let server_handle = tokio::spawn(async move {
        Server::run_server(sender, trading_pair).await;
    });
    let grpc_handle = tokio::spawn(async move {
        run_grpc(receiver).await.unwrap();
    });

    let (server_result, grpc_result) = tokio::join!(server_handle, grpc_handle);
    server_result.unwrap();
    grpc_result.unwrap();
}
