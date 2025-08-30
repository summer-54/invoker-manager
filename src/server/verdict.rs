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
        }.to_string()
    }
}

impl TestResult {
    pub fn parse_to(&self) -> String {
        format!("{} {} {}", self.verdict.parse_to(), self.time, self.memory).to_string()
    }
}
