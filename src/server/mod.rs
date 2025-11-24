mod testing_system;
mod invoker;
pub mod control_panel;
pub mod verdict;
pub mod submission;
pub mod invokers_side;
pub mod testing_system_side;

use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use invokers_side::InvokersSide;
use testing_system_side::TestingSystemSide;
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

    async fn remove_tests_result(server: Arc<Mutex<Server>>, uuid: Uuid) -> Option<Vec<TestResult>> {
        server.lock().await.tests_results.remove(&uuid)
    }
}

