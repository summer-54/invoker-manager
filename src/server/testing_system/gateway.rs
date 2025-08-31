use bytes::BytesMut;
use uuid::Uuid;
use ratchet_rs::{Error};
use super::{WSReader, WSWriter};
use crate::server::{submission::Submission, verdict::{TestResult, Verdict}};


pub struct Gateway;

impl Gateway { // wrong protocol
           
    async fn read_data_from(socket: &mut WSReader) -> Result<Vec<u8>, Error> {
        let mut bin = BytesMut::new();
        socket.read(&mut bin).await?;
        Ok(bin.to_vec())
    }

    pub async fn read_message_from(socket: &mut WSReader) -> Result<InputMessage, Error> {
        let data = Self::read_data_from(socket).await?;
        let message = InputMessage::parse(data)?;
        Ok(message)
    }

    pub async fn send_message(socket: &mut WSWriter, message: OutputMessage) -> Result<(), Error> {
        socket.write_binary(message.parse_to()).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum InputMessage {
    SubmissionRun {
        submission: Submission,
    },
}

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
    }
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

impl OutputMessage {
    fn parse_to(&self) -> Vec<u8> {
        match self {
            OutputMessage::TestVerdict { submission_uuid, test, result, data } => {
                let mut result: Vec<u8> = format!("TYPE TEST\nSUBMISSION {}\nTEST {}\nVERDICT {}\nDATA\n", submission_uuid, test, result.parse_to()).bytes().collect();
                result.append(&mut data.clone());
                result
            },
            OutputMessage::SubmissionVerdict { submission_uuid, verdict, tests_result, message } => {
                match message {
                    Ok((sum, groups)) => {
                        let mut result: Vec<u8> = format!("TYPE VERDICT\nSUBMISSION {}\nVERDICT {}\nSUM {}\nGROUPS {}\nDATA\n", submission_uuid, verdict.parse_to(), sum, groups.iter().map(|v| v.to_string()).collect::<Vec<String>>().join(" ")).bytes().collect();
                        for i in tests_result {
                            result.append(&mut format!("{}\n", i.parse_to()).bytes().collect::<Vec<u8>>());
                        }
                        result
                    },
                    Err(message) => {
                        let mut result: Vec<u8> = format!("TYPE VERDICT\nSUBMISSION {}\nVERDICT {}\nMESSAGE {}\nDATA\n", submission_uuid, verdict.parse_to(), message).bytes().collect();
                        for i in tests_result {
                            result.append(&mut format!("{}\n", i.parse_to()).bytes().collect::<Vec<u8>>());
                        }
                        result
                    }
                }
            }
        }
    }
}
