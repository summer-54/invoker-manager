use uuid::Uuid;

#[derive(Debug)]
pub struct Submission {
    pub uuid: Uuid,
    data: Vec<u8>,
}

impl Submission {
    pub fn new(uuid: Uuid, data: Vec<u8>) -> Self {
        Self {
            uuid, data,
        }
    }
}
