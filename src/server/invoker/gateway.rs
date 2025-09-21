use uuid::Uuid;
use bytes::BytesMut;
use std::{collections::HashMap, str::{self, FromStr}};
use super::{WSReader, WSWriter};
use ratchet_rs::Error;
use crate::server::{submission::Submission, verdict::{TestResult, Verdict}};

pub struct Gateway;

impl Gateway {

    pub async fn send_message_to(writer: &mut WSWriter, message: OutputMessage) -> Result<(), Error> {
        writer.write_binary(message.parse_to()).await?;
        Ok(())
    }

    async fn read_data_from(socket: &mut WSReader) -> Result<Vec<u8>, Error> {
         let mut bin = BytesMut::new();
        socket.read(&mut bin).await?;
        Ok(bin.to_vec())
    }

    fn first_line_of_bytes(data: &[u8]) -> (String, String, &[u8]) {
        let mut endl = [0; 1];
        '\n'.encode_utf8(&mut endl);
        let (first_line, data) = data.split_at(data.iter().position(|&x| x == endl[0]).unwrap_or(data.len() - 1) + 1);
        let first_line = str::from_utf8(first_line).unwrap_or("UNDEFINED").trim().to_string();
        let (message_type, first_line_arguments) = first_line.split_at(first_line.find(' ').unwrap_or(first_line.len()));
        (message_type.to_string(), first_line_arguments.to_string(), data)
    }

    pub async fn read_message_from(socket: &mut WSReader) -> Result<InputMessage, String> {
        let Ok(data) = Self::read_data_from(socket).await else {
            log::error!("Couldn't read data from socket");

            return Err("Couldn't read data from socket".to_string());
        };
        log::info!("Data readed from socket");
        let message = InputMessage::parse_from(data)?;
        log::info!("Data from socket parsed | message = {:?}", message);
        Ok(message)
    }
    pub fn to_vec_u8(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
        // TRIM()
        let Ok(string) = String::from_utf8(bytes) else {
            log::error!("Bytes couldn't be parsed to string");

            return Err("Bytes couldn't be parsed to string".to_string());
        };
        Ok(string.lines().map(|line| {u8::from_str(line).unwrap_or(0)}).collect())
    }

    pub fn parse_headers(bytes: Vec<u8>) -> (HashMap<String, String>, Vec<u8>) {
        let mut data = &bytes[..];
        let mut headers = HashMap::new();
        loop {
            if data.is_empty() {
                break;
            }
            let (key, val, bytes) = Self::first_line_of_bytes(&data);
            let key = key.trim().to_string();
            let val = val.trim().to_string();
            data = bytes;
            if key == "DATA" {
                break;
            } else {
                headers.insert(key, val);
            }
        }
        (headers, data.to_vec())
    }
}

#[derive(Debug)]
pub enum InputMessage {
    Token {
        uuid: Uuid,
    },
    Verdict {
        verdict: Verdict,
        message: Result<(u8, Vec<u8>), String>,
    },
    TestVerdict {
        result: TestResult,
        test: u16,
        data: Vec<u8>,
    },
    Exited { // don't parsed
        exit_code: String,
        exit_message: String,
    },
    Error { // don't parsed
        message: String,
    },
    OpError { // don't parsed
        message: String,
    },
}

pub enum OutputMessage {
    TestSubmission {
        submission: Submission,
    },
    StopTesting,
    CloseInvoker,
}

impl InputMessage {
    fn parse_from(bytes: Vec<u8>) -> Result<Self, String> {
        let (headers, data) = Gateway::parse_headers(bytes);
        let Some(message_type) = headers.get("TYPE") else {
            log::error!("Message doesn't contain TYPE header");

            return Err("Message doesn't contain TYPE header".to_string());
        };
        match message_type.as_str() {
            "TOKEN" => {
                let uuid = Uuid::from_str(headers.get("ID").map_or("", |s| s)).unwrap_or(Uuid::from_bytes(rand::random::<[u8; 16]>()));
                Ok(InputMessage::Token{
                    uuid,
                })
            },
            "VERDICT" => {
                let verdict = Verdict::parse(headers.get("NAME").unwrap_or(&"UV".to_string()));
                if let Verdict::UV = verdict {
                    log::error!("Readed UV verdict | headers = {:?} | data = {:?}", headers, data);
                }
                if let Verdict::OK = verdict {
                    let sum = u8::from_str(headers.get("SUM").map_or("0", |v| v)).unwrap_or(0);
                    let points = headers.get("GROUPS").cloned().unwrap_or("0".to_string()).split(" ").map(|string| u8::from_str(string).unwrap_or(0)).collect();
                    Ok(InputMessage::Verdict {
                        verdict,
                        message: Ok((sum, points)),
                    })
                } else {
                    let message = headers.get("MESSAGE").cloned().unwrap_or("Undefined error message".to_string());
                    Ok(InputMessage::Verdict {
                        verdict,
                        message: Err(message),
                    })
                }
            },
            "TEST" => {
                let test: u16 = headers.get("ID").map_or(1, |v| u16::from_str(v).unwrap_or(1));
                let verdict = Verdict::parse(&headers.get("VERDICT").cloned().unwrap_or("UV".to_string()));
                let time: f64 = headers.get("TIME").map_or(0.0, |v| f64::from_str(v).unwrap_or(0.0));
                let memory: u32 = headers.get("MEMORY").map_or(0, |v| u32::from_str(v).unwrap_or(0));
                Ok(InputMessage::TestVerdict {
                    result: TestResult {
                        verdict,
                        time,
                        memory,
                    },
                    test,
                    data,
                })
            },
            "EXITED" => {
                let exit_code = headers.get("CODE").cloned().unwrap_or("0".to_string());
                let exit_message = headers.get("MESSAGE").cloned().unwrap_or("0".to_string());
                Ok(InputMessage::Exited{
                    exit_code,
                    exit_message,
                })
            },
            "ERROR" => {
                let error = headers.get("MESSAGE").cloned().unwrap_or("".to_string());
                Ok(InputMessage::Error{
                    message: error
                })
            },
            "OPERROR" => {
                let operror = headers.get("MESSAGE").cloned().unwrap_or("".to_string());
                Ok(InputMessage::OpError{
                    message: operror
                })
            },
            &_ => Err("Can't parse message".to_string())
        }
    }
}

impl OutputMessage {
    fn parse_to(&self) -> Vec<u8> {
        match self {
            Self::TestSubmission { submission } => {
                let mut result = "TYPE START\nDATA\n".as_bytes().to_vec();
                result.append(&mut submission.data.clone());
                result
            }
            Self::StopTesting => {
                let result = "TYPE STOP\n".as_bytes().to_vec();
                result
            }
            Self::CloseInvoker => {
                let result = "TYPE CLOSE\n".as_bytes().to_vec();
                result
            }
        }
    }
}
