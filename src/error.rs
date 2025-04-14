use std::path::PathBuf;
use thiserror::Error;
use xdg::BaseDirectoriesError;

#[derive(Debug, Error)]
pub enum SRMError {
    #[error("srm: missing operand")]
    MissingOperand,

    #[error("srm: unreconised option '{0}'")]
    InvalidOption(String),

    #[error("srm: trash error: '{0}'")]
    TrashConfigError(BaseDirectoriesError),

    #[error("srm: falied to create '{0}': '{1}'")]
    TrashDirError(PathBuf, std::io::Error),

    #[error("")]
    Help,

    #[error("")]
    Version,
}

#[derive(Debug, Error)]
pub enum SRMRuntimeError {
    #[error("srm: cannot remove '{0}': {1}")]
    SrcError(PathBuf, std::io::Error),

    #[error("srm: cannot move '{0}': trash error: {1}")]
    DestError(PathBuf, std::io::Error),

    #[error("srm: failed to create trashinfo file '{0}' would not be able to use restore: {1}")]
    TrashInfoError(PathBuf, std::io::Error),
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
