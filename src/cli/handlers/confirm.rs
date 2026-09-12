//! Interactive confirmation for destructive CLI commands.

use crate::utils::{Result, TrackError};
use std::io::{self, IsTerminal, Write};

/// Reads a y/N confirmation from a TTY.
///
/// Returns [`TrackError::ConfirmationRequired`] when stdin is not a terminal so
/// agents never hang on a prompt. Pass `--force` (see `non_tty_hint`) instead.
pub fn confirm_from_tty(prompt: &str, non_tty_hint: &str) -> Result<bool> {
    confirm(io::stdin().is_terminal(), prompt, non_tty_hint, || {
        print!("{prompt}");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input)
    })
}

fn confirm(
    stdin_is_tty: bool,
    _prompt: &str,
    non_tty_hint: &str,
    read_line: impl FnOnce() -> Result<String>,
) -> Result<bool> {
    if !stdin_is_tty {
        return Err(TrackError::ConfirmationRequired {
            hint: non_tty_hint.to_string(),
        });
    }

    let input = read_line()?;
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_tty_fails_with_hint() {
        let err = confirm(false, "Delete? [y/N]: ", "re-run with `--force`", || {
            panic!("must not read stdin")
        })
        .unwrap_err();

        assert!(matches!(err, TrackError::ConfirmationRequired { .. }));
        assert!(
            err.to_string()
                .contains("Confirmation required (stdin is not a TTY)")
        );
        assert!(err.to_string().contains("re-run with `--force`"));
    }

    #[test]
    fn tty_yes_confirms() {
        let accepted =
            confirm(true, "Delete? [y/N]: ", "unused", || Ok("y\n".to_string())).unwrap();
        assert!(accepted);
    }

    #[test]
    fn tty_no_cancels() {
        let accepted =
            confirm(true, "Delete? [y/N]: ", "unused", || Ok("n\n".to_string())).unwrap();
        assert!(!accepted);
    }
}
