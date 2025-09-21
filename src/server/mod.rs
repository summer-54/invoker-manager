mod testing_system;
mod invoker;
pub mod verdict;
pub mod submission;

use std::{collections::{HashMap, HashSet}, sync::Arc};
use ratchet_rs::{SubprotocolRegistry, WebSocket, WebSocketConfig};
use ratchet_deflate::{Deflate, DeflateConfig, DeflateExtProvider};
use tokio::{net::{TcpListener, TcpStream}, sync::{mpsc, Mutex}, task::{JoinHandle}};
use uuid::Uuid;
use invoker::Invoker;
use testing_system::TestingSystem;
use invoker::gateway::{Gateway, InputMessage};
use submission::Submission;
use verdict::TestResult;

const MAX_SUBMISSIONS_COUNT: usize = 10000;

pub struct Server {
    submissions_pool_receiver: Arc<Mutex<mpsc::Receiver<Submission>>>,
    submissions_pool_sender: Arc<Mutex<mpsc::Sender<Submission>>>,
    invokers: HashMap<Uuid, Arc<Mutex<Invoker>>>,
    testing_system: Option<Arc<Mutex<TestingSystem>>>,
    tests_results: HashMap<Uuid, Vec<TestResult>>,
}

impl Server {
    pub fn new() -> Arc<Mutex<Self>> {
        let (sps, spr) = mpsc::channel::<Submission>(MAX_SUBMISSIONS_COUNT);
        Arc::new(Mutex::new(Self {
            submissions_pool_receiver: Arc::new(Mutex::new(spr)),
            submissions_pool_sender: Arc::new(Mutex::new(sps)),
            invokers: HashMap::new(),
            testing_system: None,
            tests_results: HashMap::new(),
        }))
    }

    pub async fn start_invokers_side(server: Arc<Mutex<Self>>, address: String) -> Result<(), String> {
        let Ok(listener) = TcpListener::bind(address).await else {
            log::error!("Can't bind tcp listener for invokers side");

            return Err("Can't bind tcp listener for invokers side".to_string());
        };
        log::info!("invoker_side: Binded");

        loop {
            if let Ok((connection, address)) = listener.accept().await {
                log::info!("invoker_side: Finded connection | address = {}", address);
                match ratchet_rs::accept_with(connection, WebSocketConfig::default(), DeflateExtProvider::with_config(DeflateConfig::default()), SubprotocolRegistry::default()).await {
                    Ok(stream) => {
                        log::info!("invoker_side: Finded new invoker");
                        let server = server.clone();
                        tokio::spawn(async move {
                            let Ok(upgr) = stream.upgrade().await else {
                                log::info!("invoker_side: Couldn't update to ws");
                                
                                return;
                            };
                            
                            if let Err(err) = Self::add_invoker(server, upgr.into_websocket()).await {
                                log::error!("Adding invoker falied | error = {}", err);
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
    pub async fn start_testing_system_side(server: Arc<Mutex<Self>>, ip: &str, url: &str) -> Result<(), String> {
        let testing_system = match TestingSystem::connect_to(ip, url).await {
            Ok(ts) => ts,
            Err(error) => {
                log::error!("Can't open connection to testing system side | error = {}", error.to_string());
                return Err("Can't open connection to testing system side".to_string());
            },
        };
        log::info!("testing_system_side: Started");

        server.lock().await.testing_system = Some(Arc::new(Mutex::new(testing_system)));
        if let Some(testing_system) = server.lock().await.testing_system.clone() {
            log::info!("testing_system_side: Connected to testing system");

            let result = TestingSystem::message_handler(testing_system, server.clone()).await;
            if let Err(error) = result {
                return Err(error);
            }
        } else {
            log::info!("testing_system_side: Can't connect to testing system");

            return Err("Error::ConnectionClosed".to_string())
        }
        Ok(())
    }
    pub async fn add_invoker(server: Arc<Mutex<Self>>, stream: WebSocket<TcpStream, Deflate>) -> Result<JoinHandle<Result<String, String>>, String> {
        let Ok((writer, mut reader)) = stream.split() else {
            log::error!("Stream couldn't be splited");

            return Err("Stream couldn't be splited".to_string());
        };
        log::info!("invoker: Stream splitted");
        let Ok(message) = Gateway::read_message_from(&mut reader).await else {
            log::error!("Couldn't read message from stream");

            return Err("Couldn't read message from stream".to_string());
        };
        log::info!("invoker: Sended connect message");

        if let InputMessage::Token { uuid } = message {
            let invoker = Arc::new(Mutex::new(Invoker::new(uuid, reader, writer)));
            server.lock().await.invokers.insert(uuid, invoker.clone());
            log::info!("invoker: Added | uuid = {}", uuid);

            tokio::spawn(Invoker::take_submission(invoker.clone(), server.clone()));

            Ok(tokio::spawn(Invoker::message_handler(invoker.clone(), server.clone())))
        } else {
            log::info!("invoker: doesn't sended TOKEN message");

            Err("Invoker conntected, but don't send TOKEN message first.".to_string())
        }
    }
    pub async fn new_submission(server: Arc<Mutex<Server>>, submission: Submission) -> Result<(), String> {
        let submission_uuid = submission.uuid;
        {
            let mut server_locked = server.lock().await;
            server_locked.tests_results.insert(submission_uuid, Vec::with_capacity(submission.tests_count as usize));
        }
        let submissions_pool_sender = server.lock().await.submissions_pool_sender.clone();
        if let Err(error) = submissions_pool_sender.lock().await.send(submission).await {
            if let Some(testing_system) = server.lock().await.testing_system.clone() {
                tokio::spawn(TestingSystem::send_submission_verdict(testing_system, verdict::Verdict::TE, submission_uuid, Vec::new(), Err("Couldn't add new submission to queue.".to_string())));
            }
            return Err(error.to_string());
        }
        Ok(())
    }
}

