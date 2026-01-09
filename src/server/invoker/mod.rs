pub mod gateway;

use std::sync::Arc;

use ratchet_deflate::{DeflateDecoder, DeflateEncoder};
use ratchet_rs::{Receiver, Sender};
use tokio::{net::TcpStream, sync::Mutex};
use uuid::Uuid;
pub use gateway::{Gateway, InputMessage, OutputMessage};
use super::{testing_system, Server, submission::Submission};
use invoker_auth::{policy, Challenge};

pub type WSReader = Receiver<TcpStream, DeflateDecoder>;
pub type WSWriter = Sender<TcpStream, DeflateEncoder>;

pub struct Invoker {
    uuid: Uuid,
    key: String,
    writer: Arc<Mutex<WSWriter>>,
    reader: Arc<Mutex<WSReader>>,
    submission_uuid: Option<Uuid>,
}

impl Invoker {
    pub fn new(uuid: Uuid, key: String, reader: WSReader, writer: WSWriter) -> Self {
        Self {
            uuid,
            key,
            writer: Arc::new(Mutex::new(writer)),
            reader: Arc::new(Mutex::new(reader)),
            submission_uuid: None,
        }
    }

    pub async fn authorise(invoker: Arc<Mutex<Self>>, server: Arc<Mutex<Server>>) -> Result<String, String> {
        let challenge = Challenge::generate(128, &mut rand::rng());
        log::trace!("Sending authorisation challenge");

        Gateway::send_auth_challenge(invoker.clone(), &challenge).await?;
        log::trace!("Sended authorisation challenge");

        let testing_system = server.lock().await.testing_system_side.testing_system.clone();
        // Getting certificate from testing system
        let authorisation = server.lock().await.authorisation.clone();

        let cert = authorisation.get_certificate_by_key(&invoker.lock().await.key, testing_system).await?;



        log::trace!("Gotten certificate of {}", invoker.lock().await.key);

        let reader_unlocked = invoker.lock().await.reader.clone();
        let mut reader = reader_unlocked.lock().await;
        let signed_challenge = Gateway::read_message_from(&mut reader).await?;
        log::trace!("Recieved signed_challenge message: {:?}", signed_challenge);

        if let InputMessage::SignedChallenge { bytes } = signed_challenge {
            if let Ok(()) = challenge.check_solution(&bytes, &cert, &policy::StandardPolicy::new()) {
                Gateway::send_auth_verdict(invoker, true).await?;
                Ok("Authorisation succeded".to_string())
            } else {
                Gateway::send_auth_verdict(invoker, false).await?;
                Err("Checking authorisation solutinon failed".to_string())
            }
        } else {
            Err("Wrong message after authorise".to_string())
        }
    }

    pub fn get_submission_uuid(&self) -> Option<Uuid> {
        self.submission_uuid
    }

    pub async fn delete(server: Arc<Mutex<Server>>, invoker: Arc<Mutex<Self>>) -> Result<(), String> {
        Self::finish_current_submission(server.clone(), invoker.clone()).await;
        let uuid = invoker.lock().await.uuid;
        server.lock().await.invokers_side.invokers.remove(&uuid);
        Ok(())
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

    pub async fn finish_current_submission(server: Arc<Mutex<Server>>, invoker: Arc<Mutex<Invoker>>) {
        if let Some(submission_uuid) = invoker.lock().await.submission_uuid.clone() { 
            let _ = Server::remove_tests_result(server.clone(), submission_uuid);
        } else {
            log::error!("Something went wrong, and `submission_uuid` of `Invoker` is set to None, but submission was finished.");
        }
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
                        tokio::spawn(testing_system::gateway::Gateway::send_submission_verdict(testing_system, verdict, submission_uuid, test_results, message));

                        Self::finish_current_submission(server.clone(), invoker.clone()).await;
                        match Self::take_submission(invoker.clone(), server.clone()).await {
                            Ok(Some(uuid)) => log::info!("Invoker taked new submission after completing previous | uuid = {:?} | submission_uuid = {:?}", invoker_uuid, uuid),
                            Ok(None) => log::info!("Invoker didn't take new submission after completing previous | uuid = {:?}", invoker_uuid),
                            Err(error) => log::error!("Invoker couldn't take new submission due to the error | error = {} | uuid = {:?}", error.to_string(), invoker_uuid)
                        }
                    });
                }
                InputMessage::TestVerdict { result, test, data } => {
                    {
                        log::info!("Working on TEST_VERDICT message from invoker | result = {:?} | test = {:?} | data = {:?}", result, test, data);
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
                            tokio::spawn(testing_system::gateway::Gateway::send_test_verdict(testing_system, result, test, data, submission_uuid));
                        });
                    }
                    'bl : {
                        let invoker = invoker.clone();
                        let server = server.clone();
                        let Some(submission_uuid) = invoker.lock().await.submission_uuid.clone() else {
                            log::error!("invoker_handler: Invoker sent test verdict, but hasn't current submission. | invoker_uuid: {:?}", invoker_uuid);

                            break 'bl;
                        };
                        let mut server_locked = server.lock().await;
                        let Some(tests_results) = server_locked.tests_results.get_mut(&submission_uuid) else {
                            log::error!("invoker_handler: Invoke sent test verdict, tests result isn't predefinted | invoker_uuid: {:?}", invoker_uuid);

                            break 'bl;
                        };
                        let Some(test_result) = tests_results.get_mut(test as usize - 1) else {
                            log::error!("invoker_handler: Invoker send test verdict, but current test_result is to small. | invoker_uuid: {:?} | test number = {} | currently allocated = {} | submission_uuid = {} | current map = {:?}", invoker_uuid, test - 1, tests_results.len(), submission_uuid, server_locked.tests_results);

                            break 'bl;
                        };
                        *test_result = result;
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
