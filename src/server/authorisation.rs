use std::{str::FromStr, sync::Arc};

use invoker_auth::{Cert, Parse};
use tokio::sync::Mutex;

use super::{testing_system::{self, TestingSystem}};

#[derive(Clone)]
pub enum Authorisation {
    API,
    FromFile,
    FromFileByName,
}

impl Authorisation {

    pub async fn get_certificate_by_key(&self, key: &String, testing_system: Option<Arc<Mutex<TestingSystem>>>) -> Result<Cert, String> {
        Ok(match self {
            Self::API => {
                let Some(testing_system) = testing_system else {
                    return Err("Trying to get key from API, but testing system hasn't connected yet".to_string());
                };
                testing_system::gateway::Gateway::get_certificate_by_key(testing_system, key).await?
            },
            Self::FromFile => {
                Cert::from_file("invokers_key.pub").map_err(|_| "Can't get certificate".to_string())?
            },
            Self::FromFileByName => {
                Cert::from_file(format!("invokers_key/{key}.pub")).map_err(|_| "Can't get certificate".to_string())?
            }
        })
    }

}

impl FromStr for Authorisation {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "File" | "FromFile" | "file" | "fromfile" | "FILE" | "FROMFILE" => Self::FromFile,
            "FromFileByName" | "ByName" | "fromfilebyname" | "byname" | "FROMFILEBYNAME" | "BYNAME" => Self::FromFileByName,
            _ => Self::API,
        })
    }
}
