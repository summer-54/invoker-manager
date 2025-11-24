use tokio::sync::{mpsc, Mutex};
use std::sync::Arc;
use crate::server::testing_system::TestingSystem;
use super::Server;
use super::submission::Submission;
use super::verdict::TestResult;
use super::verdict;

pub struct TestingSystemSide {
    pub testing_system: Option<Arc<Mutex<TestingSystem>>>,
    pub submissions_pool_sender: Arc<Mutex<mpsc::Sender<Submission>>>,
}

impl TestingSystemSide {
    pub fn new(sps: mpsc::Sender<Submission>) -> Self {
        Self {
            submissions_pool_sender: Arc::new(Mutex::new(sps)),
            testing_system: None,
        }
    }
    pub async fn start(server: Arc<Mutex<Server>>, ip: &str, url: &str) -> Result<(), String> {
        let testing_system = match TestingSystem::connect_to(ip, url).await {
            Ok(ts) => ts,
            Err(error) => {
                log::error!("testing_system_side: Can't open connection to testing system side | error = {} | ip = {} | url = {}", error.to_string(), ip, url);
                return Err("Can't open connection to testing system side".to_string());
            },
        };
        log::trace!("testing_system_side: Started");

        server.lock().await.testing_system_side.testing_system = Some(Arc::new(Mutex::new(testing_system)));
        if let Some(testing_system) = { server.lock().await.testing_system_side.testing_system.clone() } {
            log::trace!("testing_system_side: Connected to testing system");

            tokio::spawn(TestingSystem::pinger(testing_system.clone()));

            let result = TestingSystem::message_handler(testing_system, server.clone()).await;
            if let Err(error) = result {
                return Err(error);
            }
        } else {
            log::error!("testing_system_side: Can't connect to testing system");

            return Err("Error::ConnectionClosed".to_string())
        }
        Ok(())
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
