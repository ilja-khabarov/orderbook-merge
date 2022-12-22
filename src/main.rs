mod exchange;
mod grpc;
mod merger;
mod server;

use tracing_subscriber;

use crate::grpc::run_grpc;
use server::Server;

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
