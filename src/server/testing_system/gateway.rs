use bytes::BytesMut;
use invoker_auth::Cert;
use reqwest::Url;
use uuid::Uuid;
use std::str::FromStr;
use std::{sync::Arc, time::Duration};
use super::{WSReader, WSWriter, TestingSystem};
use ratchet_rs::{Error, HeaderValue};
use tokio::{sync::Mutex};
use crate::server::{submission::Submission, verdict::{TestResult, Verdict}};


pub struct Gateway;

impl Gateway { // wrong protocol
           
    async fn read_data_from(socket: &mut WSReader) -> Result<Vec<u8>, Error> {
        loop {
            let mut bin = BytesMut::new();
            let message = socket.read(&mut bin).await?;
            if message.is_binary() {
                log::info!("Read data from socket");
                return Ok(bin.to_vec())
            }
        }
    }

    pub async fn read_message_from(socket: &mut WSReader) -> Result<InputMessage, String> {
        let data = match Self::read_data_from(socket).await {
            Ok(data) => data,
            Err(err) => {
                return Err(format!("Can't read message from ts {:?}", err));
            }
        };
        data.try_into()
    }

    pub async fn send_message_to(socket: &mut WSWriter, message: OutputMessage) -> Result<(), String> {
        socket.write_binary::<Vec<u8>>(message.clone().into()).await.map_err(|err| err.to_string())?;
        if let OutputMessage::SubmissionVerdict { .. } = message {
            log::info!("SubmissionVerdict message sent | message = {:?}", message);
        }
        Ok(())
    }
    pub async fn send_submission_verdict(testing_system: Arc<Mutex<TestingSystem>>, verdict: Verdict, submission_uuid: Uuid, tests_result: Vec<TestResult>, message: Result<(u8, Vec<u8>), String>) {
        let writer = testing_system.lock().await.writer.clone();
        let mut writer = writer.lock().await;
        if let Err(err) = Self::send_message_to(&mut writer, OutputMessage::SubmissionVerdict{ verdict, submission_uuid, tests_result, message,
        }).await {
            log::error!("Couldn't send message | error = {}", err);
        } else {
            log::info!("testing_system: SubmissionVerdict message sent");
        }
    }
    pub async fn send_test_verdict(testing_system: Arc<Mutex<TestingSystem>>, result: TestResult, test: u16, data: Vec<u8>, submission_uuid: Uuid) {
        let writer = testing_system.lock().await.writer.clone();
        let mut writer = writer.lock().await;
        if let Err(err) = Self::send_message_to(&mut writer, OutputMessage::TestVerdict{
            result, submission_uuid, test, data
        }).await {
            log::error!("Couldn't send message | error = {:?}", err);
        } else {
            log::info!("testing_system: TestVerdict message sent");
        }
    }
    pub async fn pinger(testing_system: Arc<Mutex<TestingSystem>>) -> Result<(), Error> {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let sender = testing_system.lock().await.writer.clone();
            sender.lock().await.write_ping([0u8; 0]).await?;
        }
    }
    
    pub async fn get_certificate_by_key(testing_system: Arc<Mutex<TestingSystem>>, key: &String) -> Result<Cert, String> {
        log::trace!("Trying to get authorise key from api");
        let api_address = testing_system.lock().await.api_address.clone();
        let mut request = reqwest::Request::new(reqwest::Method::GET, Url::from_str(&format!("http://{api_address}/get_invoker_key")).map_err(|e| e.to_string())?);
        let _ = request.headers_mut().insert("Authorization", HeaderValue::from_str(key).map_err(|e| format!("Can't create header from str: {:?}", e))?);
        let client = reqwest::Client::new();
        let response = client.execute(request)
            .await.map_err(|e| format!("Can't execute request: {:?}", e))?
            .error_for_status().map_err(|e| format!("Can't do error_for_status: {:?}", e))?;
        let bytes = response.bytes().await.map_err(|e| format!("Can't get bytes: {:?}", e))?.to_vec();
        let string_response = &String::from_utf8(bytes).map_err(
            |e| format!("Can't create string from utf8 bytes from api response: {e:?}")
        )?;
        Cert::from_str(
            &string_response
        ).map_err(|e| format!("Can't get cert from string <{string_response}> : {:?}", e))
    }
}

#[derive(Debug, Clone)]
pub enum InputMessage {
    SubmissionRun {
        submission: Submission,
    },
}

#[derive(Debug, Clone)]
pub enum OutputMessage {
    TestVerdict {
        submission_uuid: Uuid,
        test: u16,
        result: TestResult,
        data: Vec<u8>,
    },
    SubmissionVerdict {
        submission_uuid: Uuid,
        verdict: Verdict,
        tests_result: Vec<TestResult>,
        message: Result<(u8, Vec<u8>), String>,
    },
}

impl TryFrom<Vec<u8>> for InputMessage {
    type Error = String;
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let data_uuid: [u8; 16] = bytes[0..16].try_into().unwrap_or([0u8; 16]);
        let uuid = Uuid::from_bytes(data_uuid);
        let test_count = u16::from_be_bytes(bytes[16..18].try_into().unwrap_or([0u8; 2]));
        let data = bytes[18..].to_vec();
        Ok(Self::SubmissionRun {
            submission: Submission::new(uuid, data, test_count),
        })
    }
}

impl From<OutputMessage> for Vec<u8> {
    fn from(value: OutputMessage) -> Vec<u8> {
        match value {
            OutputMessage::TestVerdict { submission_uuid, test, result, data } => {
                let mut result: Vec<u8> = format!("TYPE TEST\nSUBMISSION {}\nTEST {}\nVERDICT {}\nDATA\n", submission_uuid, test, String::from(result)).bytes().collect();
                result.append(&mut data.clone());
                result
            },
            OutputMessage::SubmissionVerdict { submission_uuid, verdict, tests_result, message } => {
                match message {
                    Ok((sum, groups)) => {
                        let mut result: Vec<u8> = format!("TYPE VERDICT\nSUBMISSION {}\nVERDICT {}\nSUM {}\nGROUPS {}\nDATA\n", submission_uuid, String::from(verdict), sum, groups.iter().map(|v| v.to_string()).collect::<Vec<String>>().join(" ")).bytes().collect();
                        for test_result in tests_result {
                            result.append(&mut format!("{}\n", String::from(test_result)).bytes().collect::<Vec<u8>>());
                        }
                        result
                    },
                    Err(message) => {
                        let mut result: Vec<u8> = format!("TYPE VERDICT\nSUBMISSION {}\nVERDICT {}\nMESSAGE {}\nDATA\n", submission_uuid, String::from(verdict), message).bytes().collect();
                        for test_result in tests_result {
                            result.append(&mut format!("{}\n", String::from(test_result)).bytes().collect::<Vec<u8>>());
                        }
                        result
                    }
                }
            }
        }
    }
}

