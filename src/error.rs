use std::io;
use std::path::PathBuf;
use thiserror::Error;
use xdg::BaseDirectoriesError;

#[derive(Debug, Error)]
pub enum SRMCliError {
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
pub enum SRMRuntimeErrorKind {
    #[error("source error: {0}")]
    SrcError(#[source] io::Error),

    #[error("trash error: {0}")]
    TrashError(#[source] io::Error),
}

#[derive(Debug, Error)]
pub enum SRMRuntimeError {
    #[error("failed to move '{0}' to trash: {1}")]
    MoveToTrashFailed(PathBuf, SRMRuntimeErrorKind),

    #[error("failed to write trashinfo file '{0}' : {1}")]
    TrashInfoCreationFailed(PathBuf, io::Error),
}

impl SRMRuntimeErrorKind {
    fn to_error(self, path: PathBuf) -> SRMRuntimeError {
        SRMRuntimeError::MoveToTrashFailed(path, self)
    }
}
impl SRMRuntimeError {
    pub fn new_src_error(path: PathBuf, err: io::Error) -> Self {
        let error_kind = SRMRuntimeErrorKind::SrcError(err);
        error_kind.to_error(path)
    }
    pub fn new_trash_error(path: PathBuf, err: io::Error) -> Self {
        let error_kind = SRMRuntimeErrorKind::TrashError(err);
        error_kind.to_error(path)
    }
    pub fn new_trashinfo_error(path: PathBuf, err: io::Error) -> Self {
        Self::TrashInfoCreationFailed(path, err)
    }
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
