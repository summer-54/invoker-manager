pub mod gateway;

use std::{sync::Arc, time::Duration};

use ratchet_rs::{Error, Receiver, Sender, SubprotocolRegistry, WebSocketConfig};
use ratchet_deflate::{DeflateDecoder, DeflateEncoder, DeflateExtProvider};
use gateway::{Gateway, InputMessage, OutputMessage};
use tokio::{net::TcpStream, sync::Mutex};
use uuid::Uuid;

use super::{verdict::{TestResult, Verdict}, Server, TestingSystemSide};

pub type WSReader = Receiver<TcpStream, DeflateDecoder>;
pub type WSWriter = Sender<TcpStream, DeflateEncoder>;

pub struct TestingSystem {
    writer: Arc<Mutex<WSWriter>>,
    reader: Arc<Mutex<WSReader>>,
}

impl TestingSystem {
    pub async fn connect_to(ip: &str, url: &str) -> Result<Self, Error> {
        let stream = TcpStream::connect(ip).await?;
        let socket = ratchet_rs::subscribe_with(WebSocketConfig::default(), stream, url, DeflateExtProvider::default(), SubprotocolRegistry::default()).await?.into_websocket();
        let (writer, reader) = socket.split()?;

        log::info!("testing_system_side: Connected to tssystem");
        Ok(Self::new(reader, writer))
    }
    pub fn new(reader: WSReader, writer: WSWriter) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
            reader: Arc::new(Mutex::new(reader)),
        }
    }
    pub async fn message_handler(testing_system: Arc<Mutex<Self>>, server: Arc<Mutex<Server>>) -> Result<String, String> {
        let reader = {
            testing_system.lock().await.reader.clone()
        };
        let mut reader_locked = reader.lock().await;
        'lp: loop {
            match Gateway::read_message_from(&mut reader_locked).await {
                Ok(message) => {
                    {
                        let InputMessage::SubmissionRun{submission} = message.clone();
                        log::info!("testing_system_side: Recieved a message | message = {:?}", submission.uuid);
                    }
                    match message {
                        InputMessage::SubmissionRun { submission } => {
                            tokio::spawn(TestingSystemSide::add_submission(server.clone(), submission));
                        },
                    }
                },
                Err(err) => {
                    log::error!("testing_system_side: Recieved a unparseable message | message = {:?}", err);
                    break 'lp Err("testing system sent wrong message".to_string());
                }
            }
        }
    }
    pub async fn send_submission_verdict(testing_system: Arc<Mutex<Self>>, verdict: Verdict, submission_uuid: Uuid, tests_result: Vec<TestResult>, message: Result<(u8, Vec<u8>), String>) {
        let writer = testing_system.lock().await.writer.clone();
        let mut writer = writer.lock().await;
        if let Err(err) = Gateway::send_message(&mut writer, OutputMessage::SubmissionVerdict{ verdict, submission_uuid, tests_result, message,
        }).await {
            log::error!("Couldn't send message | error = {}", err);
        } else {
            log::info!("testing_system: SubmissionVerdict message sent");
        }
    }
    pub async fn send_test_verdict(testing_system: Arc<Mutex<Self>>, result: TestResult, test: u16, data: Vec<u8>, submission_uuid: Uuid) {
        let writer = testing_system.lock().await.writer.clone();
        let mut writer = writer.lock().await;
        if let Err(err) = Gateway::send_message(&mut writer, OutputMessage::TestVerdict{
            result, submission_uuid, test, data
        }).await {
            log::error!("Couldn't send message | error = {}", err);
        } else {
            log::info!("testing_system: TestVerdict message sent");
        }
    }
    pub async fn pinger(testing_system: Arc<Mutex<Self>>) -> Result<(), Error> {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let sender = testing_system.lock().await.writer.clone();
            sender.lock().await.write_ping([0u8; 0]).await?;
        }
    }
}
