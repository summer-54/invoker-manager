#[derive(Debug, Clone)]
pub struct TestResult {
    pub verdict: Verdict,
    pub time: f64,
    pub memory: u32,
}

#[derive(Debug, Clone)]
pub enum Verdict {
    OK,
    WA,
    TL,
    RE,
    ML,
    TE, // Testing Error
    CE,
    SL, // ?
    SK, // Skipped
    PE, 
    UV,
}

impl From<&String> for Verdict {
    fn from(value: &String) -> Self {
        match value.as_str() {
            "OK" => Self::OK,
            "CE" => Self::CE,
            "TE" => Self::TE,
            "WA" => Self::WA,
            "TL" => Self::TL,
            "RE" => Self::RE,
            "ML" => Self::ML,
            "SL" => Self::SL,
            "SK" => Self::SK,
            "PE" => Self::PE,
               _ => Self::UV,
        }
    }
}

impl From<Verdict> for String {
    fn from(value: Verdict) -> Self {
        match value {
            Verdict::OK => "OK",
            Verdict::UV => "UV",
            Verdict::SL => "SL",
            Verdict::WA => "WA",
            Verdict::TL => "TL",
            Verdict::RE => "RE",
            Verdict::ML => "ML",
            Verdict::TE => "TE",
            Verdict::CE => "CE",
            Verdict::SK => "SK",
            Verdict::PE => "PE",
        }.to_string()
    }
}

impl TestResult {
    pub fn new() -> Self {
        Self {
            verdict: Verdict::SK,
            time: 0.0,
            memory: 0,
        }
    }
}

impl From<TestResult> for String {
    fn from(value: TestResult) -> String {
        let verdict: String = value.verdict.clone().into();
        format!("{} {} {}", verdict, value.time, value.memory).to_string()
    }
}
