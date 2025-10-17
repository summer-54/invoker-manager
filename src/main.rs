mod server;

use std::env;
use server::{Server, invokers_side::InvokersSide, testing_system_side::TestingSystemSide};

#[tokio::main]
async fn main() {
    env_logger::init();

    let inv_address: String = env::var("INVOKERS_ADDRESS").unwrap_or("127.0.0.1:1111".to_string());
    let ts_address: String = env::var("TS_ADDRESS").unwrap_or("127.0.0.1:2222".to_string());
    let server = Server::new();
    log::info!("Server created");
    let server_cl = server.clone();
    let inv_side = tokio::spawn(async move {
        log::info!("Invoker side started");
        if let Err(err) = InvokersSide::start(server_cl, inv_address).await {
        //if let Err(err) = Server::start_invokers_side(server_cl, "192.168.1.128:5477".to_string()).await {
            log::error!("Invokers side stoped with error | error = {}", err);
        }
    });
    let server_cl = server.clone();
    let ts_side = tokio::spawn(async move {
        log::info!("Testing system side started");
        if let Err(err) =  TestingSystemSide::start(server_cl, &ts_address, &format!("ws://{ts_address}/api/ws/setup")).await {
            log::error!("Testing system side stoped with error | error = {}", err);
        };
    });

    tokio::try_join!(inv_side, ts_side).unwrap();
}
