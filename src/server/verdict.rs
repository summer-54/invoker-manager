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
    TE,
    CE,
    SL,
    UV,
    SK,
}

impl Verdict {
    pub fn parse(verdict: &String) -> Self {
        match verdict.as_str() {
            "OK" => Self::OK,
            "CE" => Self::CE,
            "TE" => Self::TE,
            "WA" => Self::WA,
            "TL" => Self::TL,
            "RE" => Self::RE,
            "ML" => Self::ML,
            "SL" => Self::SL,
            "SK" => Self::SK,
               _ => Self::UV,
        }
    }

    pub fn parse_to(&self) -> String {
        match self {
            Self::OK => "OK",
            Self::UV => "UV",
            Self::SL => "SL",
            Self::WA => "WA",
            Self::TL => "TL",
            Self::RE => "RE",
            Self::ML => "ML",
            Self::TE => "TE",
            Self::CE => "CE",
            Self::SK => "SK",
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
    pub fn parse_to(&self) -> String {
        format!("{} {} {}", self.verdict.parse_to(), self.time, self.memory).to_string()
    }
}
