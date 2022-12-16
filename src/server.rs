use futures_util::TryFutureExt;

mod binance;
mod bitstamp;
mod client;
mod data;
mod grpc;

#[tokio::main]
async fn main() {
    //binance::do_binance().await;
    grpc::run_grpc().await.unwrap();
    //bitstamp::do_bitstamp_v2().await;
}
