pub mod gateway;

use std::sync::Arc;

use futures_util::{stream::{SplitSink, SplitStream}, StreamExt};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;
use gateway::{Gateway, InputMessage};
use super::{testing_system, Server, submission::Submission};

pub type WSReader = SplitStream<WebSocketStream<TcpStream>>;
pub type WSWriter = SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>;

pub struct Invoker {
    uuid: Uuid,
    writer: Arc<Mutex<WSWriter>>,
    reader: Arc<Mutex<WSReader>>,
    submission_uuid: Option<Uuid>,
}

impl Invoker {
    pub fn new(uuid: Uuid, reader: WSReader, writer: WSWriter) -> Self {
        Self {
            uuid,
            writer: Arc::new(Mutex::new(writer)),
            reader: Arc::new(Mutex::new(reader)),
            submission_uuid: None,
        }
    }
    pub async fn take_submission(invoker: Arc<Mutex<Invoker>>, server: Arc<Mutex<Server>>) -> Result<Option<Uuid>, String> {
        let invoker_uuid = invoker.lock().await.uuid;
        if let Some(uuid) = invoker.lock().await.submission_uuid {
            log::error!("invoker can't take submission if already taken one | uuid: {} | submission_uuid : {} ", invoker_uuid, uuid);
            return Err("Can't take submission".to_string())
        }
        let invoker_locked = invoker.lock().await;
        let submissions = server.lock().await.submissions.clone();
        if let Some(submission) =  submissions.lock().await.pop_front() {
            let submission_uuid = submission.uuid;
            
            Self::run_submission(invoker.clone(), submission);

            Ok(Some(submission_uuid))
        } else {
            Ok(None)
        }
    }
    pub async fn message_handler(invoker: Arc<Mutex<Self>>, server: Arc<Mutex<Server>>) -> Result<String, String> {
        let reader = invoker.lock().await.reader.clone();
        let invoker_uuid = invoker.lock().await.uuid;
        'lp: loop {
            let mut reader_locked = reader.lock().await;
            let message = match Gateway::read_message_from(&mut reader_locked).await {
                Ok(message) => message,
                Err(err) => {
                    log::info!("invoker_side: Recieved a message | error = {:?}", err);
                return Err("Reading error".to_string());
                }
            };
            match message {
                InputMessage::Exited { 
                    exit_code 
                } => return Ok(exit_code),
                InputMessage::Verdict { verdict, data } => {
                    let Some(submission_uuid) = invoker.lock().await.submission_uuid else {
                        log::error!("invoker_side: Invoker send VERDICT message, before taking submission");
                        continue 'lp;
                    };
                    tokio::spawn(testing_system::TestingSystem::send_message(testing_system::gateway::OutputMessage::from_invoker_message(verdict, data, submission_uuid)));
                    invoker.lock().await.submission_uuid = None;
                    tokio::spawn(Self::take_submission(invoker, server));
                }
                _ => {}
            }
        }
    }
}
