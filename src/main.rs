mod server;

use server::Server;

#[tokio::main]
async fn main() {
    env_logger::init();

    let server = Server::new("127.0.0.1:5454".to_string());
    log::info!("Server created");
    let server_cl = server.clone();
    let inv_side = tokio::spawn(async move {
        log::info!("Invoker side started");
        if let Err(err) = Server::start_invokers_side(server_cl).await {
            log::error!("Invokers side stoped with error | error = {}", err);
        }
    });
    let server_cl = server.clone();
    let ts_side = tokio::spawn(async move {
        log::info!("Testing system side started");
        if let Err(err) =  Server::start_testing_system_side(server_cl, "ws://127.0.0.1:5477").await {
            log::error!("Testing system side stoped with error | error = {}", err);
        };
    });

    tokio::try_join!(inv_side, ts_side).unwrap();
}
