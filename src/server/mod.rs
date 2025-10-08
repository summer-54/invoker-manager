mod testing_system;
mod invoker;
pub mod verdict;
pub mod submission;
pub mod invokers_side;
pub mod testing_system_side;

use std::{collections::HashMap, sync::Arc};
use ratchet_rs::{SubprotocolRegistry, WebSocket, WebSocketConfig};
use ratchet_deflate::{Deflate, DeflateConfig, DeflateExtProvider};
use tokio::{net::{TcpListener, TcpStream}, sync::{mpsc, Mutex}, task::{JoinHandle}};
use uuid::Uuid;
use invoker::Invoker;
use testing_system::TestingSystem;
use invokers_side::InvokersSide;
use testing_system_side::TestingSystemSide;
use invoker::gateway::{Gateway, InputMessage};
use submission::Submission;
use verdict::TestResult;

const MAX_SUBMISSIONS_COUNT: usize = 10000;

pub struct Server {
    pub testing_system_side: TestingSystemSide,
    pub invokers_side: InvokersSide,
    tests_results: HashMap<Uuid, Vec<TestResult>>,
}

impl Server {
    pub fn new() -> Arc<Mutex<Self>> {
        let (sps, spr) = mpsc::channel::<Submission>(MAX_SUBMISSIONS_COUNT);
        Arc::new(Mutex::new(Self {
            testing_system_side: TestingSystemSide::new(sps),
            invokers_side: InvokersSide::new(spr),
            tests_results: HashMap::new(),
        }))
    }

    pub async fn add_invoker(server: Arc<Mutex<Self>>, stream: WebSocket<TcpStream, Deflate>) -> Result<JoinHandle<Result<String, String>>, String> {
        let Ok((writer, mut reader)) = stream.split() else {
            log::error!("invoker_side: Stream couldn't be splited");

            return Err("Stream couldn't be splited".to_string());
        };
        log::trace!("invoker_side: Stream splitted");
        let Ok(message) = Gateway::read_message_from(&mut reader).await else {
            log::error!("invoker_side: Couldn't read message from stream");

            return Err("Couldn't read message from stream".to_string());
        };
        log::trace!("invoker_side: Sent connect message");

        if let InputMessage::Token { uuid } = message {
            let invoker = Arc::new(Mutex::new(Invoker::new(uuid, reader, writer)));
            {
                let mut server_locked = server.lock().await;
                server_locked.invokers_side.invokers.insert(uuid, invoker.clone());
            }
            log::trace!("invoker_side: Added | uuid = {}", uuid);

            tokio::spawn(Invoker::take_submission(invoker.clone(), server.clone()));

            Ok(tokio::spawn(Invoker::message_handler(invoker.clone(), server.clone())))
        } else {
            log::error!("invoker_side: Didn't send TOKEN message");

            Err("Invoker conntected, but don't send TOKEN message first.".to_string())
        }
    }


    pub async fn add_submission(server: Arc<Mutex<Server>>, submission: Submission) -> Result<(), String> {
        let submission_uuid = submission.uuid;
        let tests_count = submission.tests_count;
        {
            let mut server_locked = server.lock().await;
            server_locked.tests_results.insert(submission_uuid, vec![TestResult::new(); tests_count as usize]);
        }
        let submissions_pool_sender = server.lock().await.testing_system_side.submissions_pool_sender.clone();
        if let Err(error) = submissions_pool_sender.lock().await.send(submission).await {
            if let Some(testing_system) = server.lock().await.testing_system_side.testing_system.clone() {
                tokio::spawn(TestingSystem::send_submission_verdict(testing_system, verdict::Verdict::TE, submission_uuid, Vec::new(), Err("Couldn't add new submission to queue.".to_string())));
            }
            return Err(error.to_string());
        }
        log::trace!("New submission added to queue | uuid = {} | tests_count = {}", submission_uuid, tests_count);
        Ok(())
    }
}

