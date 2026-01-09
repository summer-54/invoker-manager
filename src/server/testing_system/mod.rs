pub mod gateway;
pub use gateway::{Gateway, InputMessage};

use std::sync::Arc;

use ratchet_rs::{Error, Receiver, Sender, SubprotocolRegistry, WebSocketConfig};
use ratchet_deflate::{Compression, DeflateConfig, DeflateDecoder, DeflateEncoder, DeflateExtProvider};
use tokio::{net::TcpStream, sync::Mutex};

use crate::{COMPRESSION_LEVEL, MAX_MESSAGE_SIZE};

use super::{Server, TestingSystemSide};

pub type WSReader = Receiver<TcpStream, DeflateDecoder>;
pub type WSWriter = Sender<TcpStream, DeflateEncoder>;

pub struct TestingSystem {
    writer: Arc<Mutex<WSWriter>>,
    reader: Arc<Mutex<WSReader>>,
    api_address: String,
}

impl TestingSystem {
    pub async fn connect_to(ts_ip: &str, api_addr: &str, url: &str) -> Result<Self, Error> {
        let stream = TcpStream::connect(ts_ip).await?;
        let socket = ratchet_rs::subscribe_with(
            WebSocketConfig {
                max_message_size: MAX_MESSAGE_SIZE,    // 64MB максимальный размер сообщения
            },
            stream, url,
            DeflateExtProvider::with_config(
                DeflateConfig {
                    compression_level: Compression::new(COMPRESSION_LEVEL),
                    ..Default::default()
                }
            ),
            SubprotocolRegistry::default()
        ).await?.into_websocket();

        let (writer, reader) = socket.split()?;

        log::info!("testing_system_side: Connected to tssystem");
        Ok(Self::new(reader, writer, api_addr.to_string()))
    }
    pub fn new(reader: WSReader, writer: WSWriter, api_address: String) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
            reader: Arc::new(Mutex::new(reader)),
            api_address,
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
                    match message {
                        InputMessage::SubmissionRun { submission } => {
                            log::info!("testing_system_side: Recieved a message | message = {:?}", submission.uuid);
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
}
