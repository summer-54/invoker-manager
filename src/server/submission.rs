use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Submission {
    pub uuid: Uuid,
    pub tests_count: u16,
    pub data: Vec<u8>,
    pub language: String,
    pub package_uuid: Uuid,
}

impl Submission {
    pub fn new(uuid: Uuid, data: Vec<u8>, tests_count: u16, language: String, package_uuid: Uuid) -> Self {
        Self {
            uuid, data, tests_count, language, package_uuid
        }
    }
}
