pub mod gateway;

use std::sync::Arc;

use ratchet_deflate::{DeflateDecoder, DeflateEncoder};
use ratchet_rs::{Receiver, Sender};
use tokio::{net::TcpStream, sync::Mutex};
use uuid::Uuid;
use gateway::{Gateway, InputMessage, OutputMessage};
use super::{testing_system, Server, submission::Submission};

pub type WSReader = Receiver<TcpStream, DeflateDecoder>;
pub type WSWriter = Sender<TcpStream, DeflateEncoder>;

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

    pub async fn run_submission(invoker_locked: &mut Invoker, submission: Submission) {
        invoker_locked.submission_uuid = Some(submission.uuid);
        let writer = invoker_locked.writer.clone();
        tokio::spawn(async move {
            let mut writer_locked = writer.lock().await;
            if let Err(err) = Gateway::send_message_to(&mut writer_locked, OutputMessage::TestSubmission{submission}).await {
                log::error!("Couldn't send TestSubmission message to invoker | error = {}", err);
            };
        });
    }

    pub async fn take_submission(invoker: Arc<Mutex<Invoker>>, server: Arc<Mutex<Server>>) -> Result<Option<Uuid>, String> {
        let mut invoker_locked = invoker.lock().await;
        log::info!("Invoker tries to take new submission | uuid = {}", invoker_locked.uuid);
        if let Some(uuid) = invoker_locked.submission_uuid {
            log::error!("Invoker already has submission and can't take new one | invoker_uuid = {} | submssion = {}", invoker_locked.uuid, uuid);
            return Err("Invoker already has submission and can't take new one.".to_string());
        }
        let submission = {
            /*let Ok(v) = std::fs::read("problem.tar") else {
                return Err("LITTLE ERROR".to_string());
            };
            Some(Submission::new(uuid::Uuid::from_u128(12313u128), v, 3))*/ 
            let submissions_pool_receiver_cloned = server.lock().await.invokers_side.submissions_pool_receiver.clone();
            let mut submissions_pool_receiver = submissions_pool_receiver_cloned.lock().await; // firstly we'll lock submissions, as a indicator of submissions-routing
            submissions_pool_receiver.recv().await
        };
        if let Some(submission) = submission {
            let submission_uuid = submission.uuid;
            log::info!("Invoker takes new submission | submission_uuid = {}", submission_uuid);
            Self::run_submission(&mut invoker_locked, submission).await;
            log::info!("Invoker taked new submission | submission_uuid = {}", submission_uuid);
            drop(invoker_locked);
            Ok(Some(submission_uuid))
        } else {
            log::info!("Invoker can't take new submission | uuid = {}", invoker_locked.uuid);
            drop(invoker_locked);
            Ok(None)
        }
    }

    pub async fn finish_current_submission(invoker: Arc<Mutex<Self>>) {
        invoker.lock().await.submission_uuid = None;
    }

    pub async fn message_handler(invoker: Arc<Mutex<Self>>, server: Arc<Mutex<Server>>) -> Result<String, String> {
        let reader = invoker.lock().await.reader.clone();
        let invoker_uuid = invoker.lock().await.uuid.clone();
        'lp: loop {
            let mut reader_locked = reader.lock().await;
            let message = match Gateway::read_message_from(&mut reader_locked).await {
                Ok(message) => message,
                Err(err) => {
                    log::info!("invoker_side: Recieved a message | error = {:?} | invoker_uuid = {:?}", err, invoker_uuid);
                return Err("Reading error".to_string());
                }
            };
            log::info!("invoker_handler: Recieeved message from invoker. | message = {:?} | invoker_uuid = {:?}", message, invoker_uuid);

            match message {
                InputMessage::Exited { exit_code, exit_message } => {
                    log::info!("Recieved an exit message | code = {} | message = {}", exit_code, exit_message);

                    return Ok(exit_code);
                },
                InputMessage::Verdict { verdict, message } => {
                    log::info!("Working on VERDICT message from invoker | verdict = {:?} | message = {:?}", verdict, message);
                    let Some(submission_uuid) = invoker.lock().await.submission_uuid.clone() else {
                        log::error!("invoker_side: Invoker send VERDICT message, before taking submission");
                        continue 'lp;
                    };
                    
                    let invoker = invoker.clone();
                    let server = server.clone();
                    tokio::spawn(async move {
                        let test_results = server.lock().await.tests_results.remove(&submission_uuid).unwrap_or_else(|| {
                            log::error!("invoker_handler: Undefined test results. | submission_uuid: {:?}", submission_uuid);

                            Vec::new()
                        });
                        let Some(testing_system) = server.lock().await.testing_system_side.testing_system.clone() else {
                            log::error!("invoker_handler: Recieved verdict message, but testing_systeem didn't connect. | invoker_uuid = {:?}", invoker_uuid);

                            return;
                        };
                        tokio::spawn(testing_system::TestingSystem::send_submission_verdict(testing_system, verdict, submission_uuid, test_results, message));

                        Self::finish_current_submission(invoker.clone()).await;
                        match Self::take_submission(invoker.clone(), server.clone()).await {
                            Ok(Some(uuid)) => log::info!("Invoker taked new submission after completing previous | uuid = {:?} | submission_uuid = {:?}", invoker_uuid, uuid),
                            Ok(None) => log::info!("Invoker didn't take new submission after completing previous | uuid = {:?}", invoker_uuid),
                            Err(error) => log::error!("Invoker couldn't take new submission due to the error | error = {} | uuid = {:?}", error.to_string(), invoker_uuid)
                        }
                    });
                }
                InputMessage::TestVerdict { result, test, data } => {
                    {
                        let invoker = invoker.clone();
                        let server = server.clone();
                        let result = result.clone();
                        tokio::spawn(async move {
                            let Some(submission_uuid) = invoker.lock().await.submission_uuid.clone() else {
                                log::error!("invoker_handler: Invoker sent test verdict, but hasn't current submission. | invoker_uuid: {:?}", invoker_uuid);

                                return;
                            };
                            let Some(testing_system) = server.lock().await.testing_system_side.testing_system.clone() else {
                                log::error!("invoker_handler: Recieved test verdict message, but testing_systeem didn't connect. | invoker_uuid = {:?}", invoker_uuid);

                                return;
                            };
                            tokio::spawn(testing_system::TestingSystem::send_test_verdict(testing_system, result, test, data, submission_uuid));
                        });
                    }
                    'bl : {
                        let invoker = invoker.clone();
                        let server = server.clone();
                        //tokio::spawn(async move {
                            log::warn!("A");
                            let Some(submission_uuid) = invoker.lock().await.submission_uuid.clone() else {
                                log::error!("invoker_handler: Invoker sent test verdict, but hasn't current submission. | invoker_uuid: {:?}", invoker_uuid);

                                break 'bl;
                            };
                            log::warn!("B");
                            let mut server_locked = server.lock().await;
                            let Some(tests_results) = server_locked.tests_results.get_mut(&submission_uuid) else {
                                log::error!("invoker_handler: Invoke sent test verdict, tests result isn't predefinted | invoker_uuid: {:?}", invoker_uuid);

                                break 'bl;
                            };
                            log::warn!("C");
                            let Some(test_result) = tests_results.get_mut(test as usize - 1) else {
                                log::error!("invoker_handler: Invoker send test verdict, but current test_result is to small. | invoker_uuid: {:?} | test number = {} | currently allocated = {} | submission_uuid = {} | current map = {:?}", invoker_uuid, test - 1, tests_results.len(), submission_uuid, server_locked.tests_results);

                                break 'bl;
                            };
                            log::warn!("D");
                            *test_result = result;
                        //});
                    }
                },
                InputMessage::Error { message } => {
                    log::warn!("Invoker returned error | message = {} | uuid = {}", message, invoker_uuid);
                },
                InputMessage::OpError { message } => {
                    log::warn!("Invoker returned operror | message = {} | uuid = {}", message, invoker_uuid);
                },
                _ => {}
            }
        }
    }
}
