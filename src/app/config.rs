use std::time::Duration;

use clap::Parser;

use crate::app::ui::Units;

fn parse_duration(raw: &str) -> Result<Duration, String> {
    match raw.parse::<u64>() {
        Ok(seconds) if seconds > 0 => Ok(Duration::from_secs(seconds)),
        _ => Err(format!("{} isn't a positive number", raw)),
    }
}

/// Interactive batteries viewer.
///
/// The following commands are supported while in batmon:
///
/// * Right: move to next tab
///
/// * Left: move to previous tab
///
/// * Q, Ctrl+C, Esc: close viewer
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Config {
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    /// Verbosity level, might be repeated up to 5 times (-vvvvv).
    /// Log is accessible from the stderr.
    verbose: u8,

    #[arg(
        short = 'd',
        long = "delay",
        default_value = "1",
        value_parser = parse_duration
    )]
    /// Delay between updates, in seconds
    delay: Duration,

    #[arg(
        short = 'u',
        long = "units",
        default_value = "human",
        ignore_case = true
    )]
    /// Measurement units displayed
    units: Units,
}

impl Config {
    pub fn verbosity(&self) -> usize {
        self.verbose as usize
    }

    pub fn delay(&self) -> &Duration {
        &self.delay
    }

    pub fn units(&self) -> Units {
        self.units
    }
}
