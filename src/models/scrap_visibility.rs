use serde::Serialize;
use std::fmt;
use std::str::FromStr;

/// Whether a scrap is local-only or included in the published work record.
///
/// Scraps default to [`Local`]. Shared scraps are projected as git notes on
/// the matching TODO's commit (`refs/notes/track`) when aggressive mode is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScrapVisibility {
    #[default]
    Local,
    Shared,
}

impl ScrapVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Shared => "shared",
        }
    }

    pub fn is_shared(self) -> bool {
        matches!(self, Self::Shared)
    }

    pub fn from_db(value: i64) -> Self {
        if value == 0 {
            Self::Local
        } else {
            Self::Shared
        }
    }

    pub fn as_db(self) -> i64 {
        match self {
            Self::Local => 0,
            Self::Shared => 1,
        }
    }
}

impl fmt::Display for ScrapVisibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ScrapVisibility {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "local" | "private" => Ok(Self::Local),
            "shared" | "share" | "public" => Ok(Self::Shared),
            other => Err(format!(
                "unknown scrap visibility '{other}' (expected 'local' or 'shared')"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_visibility() {
        assert_eq!(
            "local".parse::<ScrapVisibility>().unwrap(),
            ScrapVisibility::Local
        );
        assert_eq!(
            "shared".parse::<ScrapVisibility>().unwrap(),
            ScrapVisibility::Shared
        );
        assert!("maybe".parse::<ScrapVisibility>().is_err());
        assert_eq!(ScrapVisibility::from_db(0), ScrapVisibility::Local);
        assert_eq!(ScrapVisibility::from_db(1), ScrapVisibility::Shared);
    }
}
