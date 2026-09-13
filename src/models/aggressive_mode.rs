use serde::Serialize;
use std::fmt;
use std::str::FromStr;

/// Whether track creates a per-task VCS revision and stores scraps as git notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AggressiveMode {
    #[default]
    Off,
    On,
}

impl AggressiveMode {
    pub const KEY: &'static str = "aggressive_mode";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::On => "on",
        }
    }

    pub fn is_on(self) -> bool {
        matches!(self, Self::On)
    }
}

impl fmt::Display for AggressiveMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AggressiveMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "on" | "true" | "1" | "yes" | "enabled" => Ok(Self::On),
            "off" | "false" | "0" | "no" | "disabled" => Ok(Self::Off),
            other => Err(format!(
                "unknown aggressive mode '{other}' (expected 'on' or 'off')"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_aggressive_mode() {
        assert_eq!("on".parse::<AggressiveMode>().unwrap(), AggressiveMode::On);
        assert_eq!(
            "true".parse::<AggressiveMode>().unwrap(),
            AggressiveMode::On
        );
        assert_eq!(
            "off".parse::<AggressiveMode>().unwrap(),
            AggressiveMode::Off
        );
        assert_eq!(
            "false".parse::<AggressiveMode>().unwrap(),
            AggressiveMode::Off
        );
        assert!("maybe".parse::<AggressiveMode>().is_err());
    }
}
