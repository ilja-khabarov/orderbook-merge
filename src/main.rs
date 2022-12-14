mod binance;
mod bitstamp;
mod client;
mod data;

#[tokio::main]
async fn main() {
    //binance::do_binance().await;
    bitstamp::do_bitstamp_v2().await;
}
