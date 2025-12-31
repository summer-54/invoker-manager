mod server;

use std::env;
use server::{Server, invokers_side::InvokersSide, testing_system_side::TestingSystemSide, control_panel::ControlPanel};

pub const MAX_MESSAGE_SIZE: usize = 1 << 31;
pub const COMPRESSION_LEVEL: u32 = 9;

#[tokio::main]
async fn main() {
    env_logger::init();

    let inv_address: String = env::var("INVOKERS_ADDRESS").unwrap_or("127.0.0.1:1111".to_string());
    let ts_address: String = env::var("TS_ADDRESS").unwrap_or("127.0.0.1:2222".to_string());
    let cp_address: String = env::var("CP_ADDRESS").unwrap_or("127.0.0.1:3333".to_string());

    let server = Server::new();
    log::info!("Server created");
    let inv_side = {
        let server = server.clone();
        tokio::spawn(async move {
            log::info!("Invoker side started");
            if let Err(err) = InvokersSide::start(server, inv_address).await {
            //if let Err(err) = Server::start_invokers_side(server_cl, "192.168.1.128:5477".to_string()).await {
                log::error!("Invokers side stoped with error | error = {}", err);
            }
        })
    };
    let ts_side = {
        let server = server.clone();
        tokio::spawn(async move {
            log::info!("Testing system side started");
            if let Err(err) =  TestingSystemSide::start(server, &ts_address, &format!("ws://{ts_address}/api/ws/setup")).await {
                log::error!("Testing system side stoped with error | error = {}", err);
            };
        })
    };
    let control_panel = {
        let server = server.clone();
        tokio::spawn(async move {
            let control_panel = match ControlPanel::binded_to(&cp_address, server).await {
                Ok(cp) => cp,
                Err(er) => {
                    log::error!("Control panel hasn't binded: {er}");
                    return ();
                }
            };
            if let Err(err) = control_panel.start_listening().await {
                log::error!("Error of control panel occure {err}.")
            }
        })
    };

    tokio::try_join!(inv_side, ts_side, control_panel).unwrap();
}

