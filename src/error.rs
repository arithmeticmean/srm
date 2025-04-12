use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RMSCliError {
    #[error("rms: missing operand")]
    MissingOperand,

    #[error("rms: unreconised option '{0}'")]
    InvalidOption(String),

    #[error("")]
    Help,

    #[error("")]
    Version,
}

#[derive(Debug, Error)]
pub enum RMSError {
    #[error("rms: cannot remove '{0}': {1}")]
    SrcError(PathBuf, std::io::Error),

    #[error("rms: cannot move '{0}': {1}")]
    DestError(PathBuf, std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExitCode {
    Success = 0,
    RuntimeError = 1,
    UsageError = 2,
}

impl ExitCode {
    pub fn code(self) -> i32 {
        self as i32
    }

    pub fn update(&mut self, new: ExitCode) {
        *self = std::cmp::max(*self, new);
    }
}
