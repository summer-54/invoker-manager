use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Submission {
    pub uuid: Uuid,
    pub tests_count: u16,
    pub data: Vec<u8>,
}

impl Submission {
    pub fn new(uuid: Uuid, data: Vec<u8>, tests_count: u16) -> Self {
        Self {
            uuid, data, tests_count,
        }
    }
}
