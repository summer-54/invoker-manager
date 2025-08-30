use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::{Error, Message};
use uuid::Uuid;
use std::{io, str};
use super::{WSReader, WSWriter};
use crate::server::{submission::Submission, verdict::{TestResult, Verdict}};


pub struct Gateway;

impl Gateway { // wrong protocol
    async fn read_data_from(socket: &mut WSReader) -> Result<Vec<u8>, Error> {
        let Some(bin) = socket.next().await else {
            return Err(Error::Io(io::Error::new(io::ErrorKind::NotConnected , "Can't read message")));
        };
        Ok(bin?.into_data().to_vec())
    }

    fn first_line_of_bytes(data: Vec<u8>) -> (String, String, Vec<u8>) {
        let mut endl = [0; 1];
        '\n'.encode_utf8(&mut endl);
        let (first_line, data) = data.split_at(data.iter().position(|&x| x == endl[0]).unwrap_or(data.len()));
        let first_line = str::from_utf8(first_line).unwrap_or("UNDEFINED").to_string();
        let (message_type, first_line_arguments) = first_line.split_at(first_line.find(' ').unwrap_or(first_line.len()));
        (message_type.to_string(), first_line_arguments.to_string(), data.to_vec())
    }

    pub async fn read_message_from(socket: &mut WSReader) -> Result<InputMessage, Error> {
        let data = Self::read_data_from(socket).await?;
        let message = InputMessage::parse(data)?;
        Ok(message)
    }

    pub async fn send_message(socket: &mut WSWriter, message: OutputMessage) -> Result<(), Error> {
        socket.send(Message::binary(message.parse_to())).await?;
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
        todo!();
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
