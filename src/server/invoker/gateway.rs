use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::Error;
use uuid::Uuid;
use std::{io, str};
use super::WSReader;
use crate::server::verdict::Verdict;

pub struct Gateway;

impl Gateway {

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
}

pub enum InputMessage {
    Token {
        uuid: Uuid,
    },
    Verdict {
        verdict: Verdict,
        data: Vec<u8>,
    },
    TestVerdict {
        verdict: Verdict,
        test: u16,
        data: Vec<u8>,
    },
    Exited { // don't parsed
        exit_code: String,
    },
    Error { // don't parsed
        message: String,
    },
    OpError { // don't parsed
        message: String,
    },
}

impl InputMessage {
    fn parse(bytes: Vec<u8>) -> Result<Self, Error> {
        let (message_type, arguments, data) = Gateway::first_line_of_bytes(bytes);
        match message_type.as_str() {
            "TOKEN" => {
                Ok(InputMessage::Token{
                    uuid: Uuid::from_bytes(data.try_into().unwrap_or(rand::random::<[u8; 16]>())),
                })
            },
            "VERDICT" => {
                Ok(InputMessage::Verdict {
                    verdict: Verdict::parse(&arguments),
                    data,
                })
            },
            "TEST" => {
                let test: u32 = arguments.parse().unwrap_or(0);
                let (_, verdict, data) = Gateway::first_line_of_bytes(data);
                Ok(InputMessage::TestVerdict {
                    verdict: Verdict::parse(&verdict),
                    test,
                    data,
                })
            },
            &_ => Err(Error::Io(io::Error::new(io::ErrorKind::InvalidData, "Can't parse message")))
        }
    }
}
