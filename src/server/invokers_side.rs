use tokio::{net::{TcpListener, TcpStream}, sync::{mpsc, Mutex}, task::JoinHandle};
use std::{collections::HashMap, sync::Arc};
use ratchet_rs::{SubprotocolRegistry, WebSocketConfig, WebSocket};
use ratchet_deflate::{Compression, Deflate, DeflateConfig, DeflateExtProvider};
use crate::{server::invoker::Invoker, COMPRESSION_LEVEL, MAX_MESSAGE_SIZE};
use super::invoker::gateway::Gateway as InvokerGateway;
use super::invoker::gateway::InputMessage as InvokerInMessage;
use uuid::Uuid;
use super::submission::Submission;

use super::Server;

pub struct InvokersSide {
    pub submissions_pool_receiver: Arc<Mutex<mpsc::Receiver<Submission>>>,
    pub invokers: HashMap<Uuid, Arc<Mutex<Invoker>>>,
}

impl InvokersSide {
    pub fn new(spr: mpsc::Receiver<Submission>) -> Self {
        Self {
            submissions_pool_receiver: Arc::new(Mutex::new(spr)),
            invokers: HashMap::new(),
        }
    }

    pub async fn start(server: Arc<Mutex<Server>>, address: String) -> Result<(), String> {
        let Ok(listener) = TcpListener::bind(address).await else {
            log::error!("invoker_side: Can't bind tcp listener for invokers side");

            return Err("Can't bind tcp listener for invokers side".to_string());
        };
        log::trace!("invoker_side: Binded");

        loop {
            if let Ok((connection, address)) = listener.accept().await {
                log::trace!("invoker_side: Finded connection | address = {}", address);
                match ratchet_rs::accept_with(connection,
                    WebSocketConfig {
                        max_message_size: MAX_MESSAGE_SIZE,
                    },
                    DeflateExtProvider::with_config(DeflateConfig {
                        compression_level: Compression::new(COMPRESSION_LEVEL),
                        ..Default::default()
                    }),
                    SubprotocolRegistry::default()
                ).await {
                    Ok(stream) => {
                        log::trace!("invoker_side: Found new invoker");
                        let server = server.clone();
                        tokio::spawn(async move {
                            let Ok(upgr) = stream.upgrade().await else {
                                log::error!("invoker_side: Couldn't update to ws");
                                
                                return;
                            };
                            
                            if let Err(err) = Self::add_invoker(server, upgr.into_websocket()).await {
                                log::error!("invoker_side: Adding invoker falied | error = {}", err);
                            };
                        });
                    },
                    Err(err) => {
                        log::error!("invoker_side: Failed connection | error = {}", err);
                    }
                }
            }
        }
    }
    pub async fn add_invoker(server: Arc<Mutex<Server>>, stream: WebSocket<TcpStream, Deflate>) -> Result<JoinHandle<Result<String, String>>, String> {
        let Ok((writer, mut reader)) = stream.split() else {
            log::error!("invoker_side: Stream couldn't be splited");

            return Err("Stream couldn't be splited".to_string());
        };
        log::trace!("invoker_side: Stream splitted");
        let Ok(message) = InvokerGateway::read_message_from(&mut reader).await else {
            log::error!("invoker_side: Couldn't read message from stream");

            return Err("Couldn't read message from stream".to_string());
        };
        log::trace!("invoker_side: Sent connect message");

        if let InvokerInMessage::Token { uuid } = message {
            let invoker = Arc::new(Mutex::new(Invoker::new(uuid, reader, writer)));
            {
                let mut server_locked = server.lock().await;
                server_locked.invokers_side.invokers.insert(uuid, invoker.clone());
            }
            log::trace!("invoker_side: Added | uuid = {}", uuid);

            tokio::spawn(Invoker::take_submission(invoker.clone(), server.clone()));

            Ok(tokio::spawn(async move {
                let result = Invoker::message_handler(invoker.clone(), server.clone()).await;
                Invoker::delete(server, invoker).await?;

                log::trace!("invoker_side: Removed | uuid = {}", uuid);
                result
            }))
        } else {
            log::error!("invoker_side: Didn't send TOKEN message");

            Err("Invoker conntected, but don't send TOKEN message first.".to_string())
        }
    }
    pub async fn delete_invoker(server: Arc<Mutex<Server>>, uuid: Uuid) -> Result<(), String> {
        let invoker = {
            let server_locked = server.lock().await;
            let Some(invoker) = server_locked.invokers_side.invokers.get(&uuid) else {
                return Err(format!("Invoker {uuid} doesn't exist"));
            };
            invoker.clone()
        };

        tokio::spawn(Invoker::take_submission(invoker.clone(), server.clone()));
        Invoker::delete(server.clone(), invoker.clone()).await?;
        return Ok(());
    }
    pub async fn get_invokers_status(&self) -> HashMap<Uuid, Option<Uuid>> {
        let invokers_side = self.invokers.clone();
        let mut map = HashMap::new();
        for (uuid, invoker) in invokers_side {
            map.insert(uuid, invoker.lock().await.get_submission_uuid());
        }
        map
    }
}
