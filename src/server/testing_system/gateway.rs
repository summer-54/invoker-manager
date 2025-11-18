use std::collections::HashMap;

use bytes::BytesMut;
use uuid::Uuid;
use ratchet_rs::{Error};
use super::{WSReader, WSWriter};
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

    pub async fn read_message_from(socket: &mut WSReader) -> Result<InputMessage, Error> {
        let data = Self::read_data_from(socket).await?;
        let message = InputMessage::parse(data)?;
        Ok(message)
    }

    pub async fn send_message(socket: &mut WSWriter, message: OutputMessage) -> Result<(), Error> {
        socket.write_binary::<Vec<u8>>(message.clone().into()).await?;
        if let OutputMessage::SubmissionVerdict { .. } = message {
            log::info!("SubmissionVerdict message sent | message = {:?}", message);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum InputMessage {
    SubmissionRun {
        submission: Submission,
    },
    InvokersStatus {
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
    InvokersStatus {
        invokers_tasks: HashMap<Uuid, Option<Uuid>>,
    },
}

impl InputMessage {
    fn parse(bytes: Vec<u8>) -> Result<Self, Error> {
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
            },
            OutputMessage::InvokersStatus { invokers_tasks } => {
                let mut result: Vec<u8> = format!("TYPE INVOKERS_STATUS\nDATA\n").bytes().collect();
                for (invoker_uuid, task_uuid) in invokers_tasks {
                    result.append(&mut invoker_uuid.into_bytes().to_vec());
                    result.append(&mut task_uuid.unwrap_or(Uuid::nil()).into_bytes().to_vec());
                }
                result
            }
        }

    }
}

