mod testing_system;
mod invoker;
pub mod verdict;
pub mod submission;

use std::{collections::{HashMap, VecDeque}, io, sync::Arc};
use futures_util::StreamExt;
use tokio::{net::{TcpListener, TcpStream}, sync::Mutex, task::{JoinHandle}};
use tokio_tungstenite::{tungstenite::{Error}, WebSocketStream};
use uuid::Uuid;
use invoker::Invoker;
use testing_system::TestingSystem;
use invoker::gateway::{Gateway, InputMessage};
use submission::Submission;

pub struct Server {
    address: String,
    submissions: Arc<Mutex<VecDeque<Submission>>>,
    invokers: HashMap<Uuid, Arc<Mutex<Invoker>>>,
    testing_system: Option<Arc<Mutex<TestingSystem>>>,
    unload_invokers_uuid: VecDeque<Uuid>,
}

impl Server {
    pub fn new(address: String) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            address,
            submissions: Arc::new(Mutex::new(VecDeque::new())),
            invokers: HashMap::new(),
            testing_system: None,
            unload_invokers_uuid: VecDeque::new(),
        }))
    }

    pub async fn start_invokers_side(server: Arc<Mutex<Self>>) -> Result<(), Error> {
        let listener = TcpListener::bind(server.lock().await.address.clone()).await?;
        log::info!("invoker_side: Binded");

        loop {
            let (connection, _) = listener.accept().await?;
            log::info!("invoker_side: Finded connection");

            if let Ok(stream) = tokio_tungstenite::accept_async(connection).await {
                log::info!("invoker_side: Finded new invoker");

                tokio::spawn(Self::add_invoker(server.clone(), stream));
            }
        }
    }
    pub async fn start_testing_system_side(server: Arc<Mutex<Self>>) -> Result<(), Error> {
        let testing_system = TestingSystem::connect_to("wss://").await?;
        log::info!("testing_system_side: Started");

        server.lock().await.testing_system = Some(Arc::new(Mutex::new(testing_system)));
        if let Some(testing_system) = server.lock().await.testing_system.clone() {
            log::info!("testing_system_side: Connected to testing system");

            TestingSystem::message_handler(testing_system, server.clone()).await;
        } else {
            log::info!("testing_system_side: Can't connect to testing system");

            return Err(Error::ConnectionClosed)
        }
        Ok(())
    }
    pub async fn add_invoker(server: Arc<Mutex<Self>>, stream: WebSocketStream<TcpStream>) -> Result<JoinHandle<Result<String, String>>, Error> {
        let (writer, mut reader) = stream.split();
        let message = Gateway::read_message_from(&mut reader).await?;
        log::info!("invoker: Sended connect message");

        if let InputMessage::Token { uuid } = message {
            let invoker = Arc::new(Mutex::new(Invoker::new(uuid, reader, writer)));
            server.lock().await.invokers.insert(uuid, invoker.clone());
            log::info!("invoker: Added | uuid = {}", uuid);
            
            Ok(tokio::spawn(Invoker::message_handler(invoker.clone(), server.clone())))
        } else {
            log::info!("invoker: doesn't sended TOKEN message");

            Err(Error::Io(io::Error::new(io::ErrorKind::NotConnected, "Invoker conntected, but don't send TOKEN message first.")))
        }
    }
    pub async fn new_submission(server: Arc<Mutex<Server>>, submission: Submission) {
        let submission_queue = server.lock().await.submissions.clone();
        submission_queue.lock().await.push_back(submission);
    }
}

