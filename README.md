# orderbook-merge

## Code structure:
/client - trivial client, which subscribes to GRPC and prints everything to stdout
/src - server sources
/src/server - main service, running exchange clients
/src/exchange - code related to connection to exchanges
/src/grpc - code related to grpc service
/src/merger - logic behind the actual merging
/src/main.rs - main, obviously. Runs server and grpc services and provides a channel to connect them

## commands:
`cargo run --bin orderbook-server --release` - run server
`cargo run --bin orderbook-client --release` - run client
