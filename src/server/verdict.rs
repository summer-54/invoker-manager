#[derive(Debug)]
pub struct TestResult {
    verdict: Verdict,
}
pub enum Verdict {
    OK,
    WA,
    TL,
    RE,
    ML,
    TE,
    CE,
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
               _ => Self::UV,
        }
    }
}
