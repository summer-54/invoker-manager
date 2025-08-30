pub mod gateway;

use std::sync::Arc;

use futures_util::{stream::{SplitSink, SplitStream}, StreamExt};
use gateway::{Gateway, InputMessage, OutputMessage};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::{connect_async, tungstenite::{client::IntoClientRequest, http::HeaderValue, Error}, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;
use super::{verdict::{TestResult, Verdict}, Server};


pub type WSReader = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
pub type WSWriter = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tokio_tungstenite::tungstenite::Message>;

pub struct TestingSystem {
    writer: Arc<Mutex<WSWriter>>,
    reader: Arc<Mutex<WSReader>>,
}

impl TestingSystem {
    pub async fn connect_to(url: &str) -> Result<Self, Error> {
        let mut request = url.into_client_request()?;
        request.headers_mut().insert("a", HeaderValue::from_str("b")?);
        let (stream, responce) = connect_async(request.clone()).await?;
        let (writer, reader) = stream.split();
        log::info!("testing_system_side: Connected to tssystem | response = {:?} | request = {:?} ", responce, request);
        Ok(Self::new(reader, writer))
    }
    pub fn new(reader: WSReader, writer: WSWriter) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
            reader: Arc::new(Mutex::new(reader)),
        }
    }
    pub async fn message_handler(testing_system: Arc<Mutex<Self>>, server: Arc<Mutex<Server>>) -> Result<String, String> {
        let reader = testing_system.lock().await.reader.clone();
        let mut reader_locked = reader.lock().await;
        loop {
            match Gateway::read_message_from(&mut reader_locked).await {
                Ok(message) => {
                    log::info!("testing_system_side: Recieved a message | message = {:?}", message);
                    match message {
                        InputMessage::SubmissionRun { submission } => {
                            tokio::spawn(Server::new_submission(server.clone(), submission));
                        },
                    }
                },
                Err(err) => log::error!("testing_system_side: Recieved a unparseable message | message = {:?}", err),
            }
        }
    }
    pub async fn send_submission_verdict(testing_system: Arc<Mutex<Self>>, verdict: Verdict, submission_uuid: Uuid, tests_result: Vec<TestResult>, message: Result<(u8, Vec<u8>), String>) {
        let writer = testing_system.lock().await.writer.clone();
        let mut writer = writer.lock().await;
        Gateway::send_message(&mut writer, OutputMessage::SubmissionVerdict{
            verdict, submission_uuid, tests_result, message,
        }).await;
        log::info!("testing_system: SubmissionVerdict message sent")
    }
    pub async fn send_test_verdict(testing_system: Arc<Mutex<Self>>, result: TestResult, test: u16, data: Vec<u8>, submission_uuid: Uuid) {
        let writer = testing_system.lock().await.writer.clone();
        let mut writer = writer.lock().await;
        Gateway::send_message(&mut writer, OutputMessage::TestVerdict{
            result, submission_uuid, test, data
        }).await;
        log::info!("testing_system: TestVerdict message sent")
    }
}
