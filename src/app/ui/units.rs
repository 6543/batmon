use std::str::FromStr;

use clap::ValueEnum;

use crate::Error;

#[derive(Debug, Eq, PartialEq, Copy, Clone, ValueEnum)]
pub enum Units {
    Human,
    Si,
}

impl FromStr for Units {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match () {
            _ if s.eq_ignore_ascii_case("human") => Ok(Units::Human),
            _ if s.eq_ignore_ascii_case("si") => Ok(Units::Si),
            _ => Err(Error::ParseError),
        }
    }
}
