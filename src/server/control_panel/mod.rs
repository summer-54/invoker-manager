use std::sync::Arc;

use tokio::{net::TcpListener, sync::Mutex};
use uuid::Uuid;
use std::collections::HashMap;
use axum::{extract::{State, Path}, response::IntoResponse, routing::{get, delete}, Router};

use super::{invokers_side::InvokersSide, verdict::TestResult, Server};

pub struct ControlPanel {
    listener: TcpListener,
    app: Router,
}

impl ControlPanel {
    pub async fn binded_to(ip: &str, server: Arc<Mutex<Server>>) -> Result<Self, String> {
        let app = Router::new()
            .nest("/control-panel", control_panel_handler())
            .with_state(server);

        Ok(Self {
            app,
            // run our app with hyper, listening globally on port 3000
            listener: match tokio::net::TcpListener::bind(ip).await {
                Ok(listener) => listener,
                Err(err) => return Err(format!("Can't bind tcp listener to {ip}: {err:?}")),
            }
        })
    }

    pub async fn start_listening(self) -> Result<String, String> {
        log::trace!("Control panel start listeneing.");
        match axum::serve(self.listener, self.app).await { Ok(()) => Ok("Serving succesfuly ended".to_string()),
            Err(err) => Err(format!("Occure error while serving: {err:?}.")),
        }
    }
}

async fn get_invokers_status_handler(State(server): State<Arc<Mutex<Server>>>) -> impl IntoResponse {
    log::trace!("Recieved GET invokers-status.");
    let invokers_status = server.lock().await.invokers_side.get_invokers_status().await;
    let map: HashMap<String, Option<String>> = invokers_status.iter().map(
        |(key, val)| (
        key.to_string(),
        if let Some(id) = val {
            Some(id.to_string())
        } else {
            None
        })
    ).collect();
    

    match serde_json::to_string(&map) {
        Ok(string) => {
            log::trace!("Sending invokers-status: {string}");
            string
        },
        Err(err) => {
            log::error!("Failed to parse map of invokers_tasks {map:?} to string: {err:?}");
            "SERVERERROR".to_string()
        }
    }
}

async fn get_tests_results_handler(State(server): State<Arc<Mutex<Server>>>) -> impl IntoResponse {
    let tests_results = server.lock().await.tests_results.clone();
    let map: HashMap<String, Vec<TestResult>> = tests_results.iter().map(
        |(key, val)| 
        (key.to_string(), val.clone())
    ).collect();
 
    match serde_json::to_string(&map) {
        Ok(string) => {
            log::trace!("Sending tests_results {string}");
            string
        },
        Err(err) => {
            log::error!("Failed to parse map of tests_results {map:?} to string: {err:?}");
            "SERVERERROR".to_string()
        }
    }   
}

async fn delete_invoker_handler(Path(path): Path<String>, State(server): State<Arc<Mutex<Server>>>) -> impl IntoResponse {
    let invoker_uuid = match Uuid::try_from(path) {
        Ok(invoker_uuid) => invoker_uuid,
        Err(_err) => {
            return "{invoker_uuid} doesn't parse to uuid.".to_string();
        }
    };

    if let Err(err) = InvokersSide::delete_invoker(server, invoker_uuid).await {
        return err;
    }
    "Succes".to_string()
}

fn control_panel_handler() -> Router<Arc<Mutex<Server>>> {
    Router::<Arc<Mutex<Server>>>::new()
        .route("/invokers-status", get(get_invokers_status_handler))
        .route("/tests-results", get(get_tests_results_handler))
        .route("/invokers/{invoker_uuid}", delete(delete_invoker_handler))
}
