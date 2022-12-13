mod client;
mod binance;
mod bitstamp;

#[tokio::main]
async fn main() {
    binance::do_binance().await;
    //bitstamp::do_bitstamp().await;
}