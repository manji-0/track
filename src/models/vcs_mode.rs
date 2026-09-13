use serde::Serialize;
use std::fmt;
use std::str::FromStr;

/// Version control backend for task workspaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VcsMode {
    /// Plain git worktrees and branches. Default.
    #[default]
    Git,
    /// Jujutsu (colocated with git). Track owns `jj workspace add`.
    Jj,
}

impl VcsMode {
    pub const KEY: &'static str = "vcs_mode";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Jj => "jj",
            Self::Git => "git",
        }
    }
}

impl fmt::Display for VcsMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for VcsMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "jj" | "jujutsu" => Ok(Self::Jj),
            "git" => Ok(Self::Git),
            other => Err(format!(
                "unknown vcs mode '{other}' (expected 'jj' or 'git')"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_vcs_mode() {
        assert_eq!("jj".parse::<VcsMode>().unwrap(), VcsMode::Jj);
        assert_eq!("git".parse::<VcsMode>().unwrap(), VcsMode::Git);
        assert!("svn".parse::<VcsMode>().is_err());
    }
}
