mod server;

use server::Server;

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("Invoker manager started");

    let server = Server::new("127.0.0.1:5454".to_string());
    log::info!("Server created");

    let invokers = Server::start_invokers_side(server.clone());
    log::info!("Invoker side started");

    let testing_system = Server::start_testing_system_side(server.clone());
    log::info!("Testing system side started");

    tokio::try_join!(invokers, testing_system).unwrap();
    log::info!("Invoker manager stopped");
}
