use std::sync::Arc;

use tokio::{net::{TcpListener, TcpStream}, sync::Mutex};
use std::collections::HashMap;
use axum::{Router, routing::get};

use super::Server;

pub type Reader = TcpStream;
pub type Writer = TcpStream;

pub struct ControlPanel {
    listener: TcpListener,
    app: Router,
}

impl ControlPanel {
    pub async fn binded_to(ip: &str, server: Arc<Mutex<Server>>) -> Result<Self, String> {
        let mut app = Router::new();

        { // invokers_status
            let server = server.clone();
            app = app.route("/control-panel/invokers-status", get(|| async move { 
                log::trace!("Recieved GET invokers-status.");
                let invokers_status = server.lock().await.invokers_side.get_invokers_status().await;
                let map: HashMap<String, Option<String>> = invokers_status.iter().map(
                    |(key, val)| 
                    (uuid::fmt::Urn::from_uuid(key.clone()).to_string(), if let Some(id) = val {
                        Some(uuid::fmt::Urn::from_uuid(id.clone()).to_string())
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
            }));
        }

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
            Err(err) => Err(format!("Occure error: {err:?}.")),
        }
    }
}
