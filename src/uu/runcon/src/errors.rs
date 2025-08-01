// This file is part of the uutils coreutils package.
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed with this source code.
#![cfg(target_os = "linux")]

use std::ffi::OsString;
use std::fmt::{Display, Formatter, Write};
use std::io;
use std::str::Utf8Error;

use uucore::display::Quotable;
use uucore::error::UError;
use uucore::translate;

pub(crate) type Result<T> = std::result::Result<T, Error>;

// This list is NOT exhaustive. This command might perform an `execvp()` to run
// a different program. When that happens successfully, the exit status of this
// process will be the exit status of that program.
pub(crate) mod error_exit_status {
    pub const NOT_FOUND: i32 = 127;
    pub const COULD_NOT_EXECUTE: i32 = 126;
    pub const ANOTHER_ERROR: i32 = libc::EXIT_FAILURE;
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("{}", translate!("runcon-error-no-command"))]
    MissingCommand,

    #[error("{}", translate!("runcon-error-selinux-not-enabled"))]
    SELinuxNotEnabled,

    #[error(transparent)]
    NotUTF8(#[from] Utf8Error),

    #[error(transparent)]
    CommandLine(#[from] clap::Error),

    #[error("{}", translate!("runcon-error-operation-failed", "operation" => .operation))]
    SELinux {
        operation: &'static str,
        source: selinux::errors::Error,
    },

    #[error("{}", translate!("runcon-error-operation-failed", "operation" => .operation))]
    Io {
        operation: &'static str,
        source: io::Error,
    },

    #[error("{}", translate!("runcon-error-operation-failed-on", "operation" => .operation, "operand" => .operand1.quote()))]
    Io1 {
        operation: &'static str,
        operand1: OsString,
        source: io::Error,
    },
}

impl Error {
    pub(crate) fn from_io(operation: &'static str, source: io::Error) -> Self {
        Self::Io { operation, source }
    }

    pub(crate) fn from_io1(
        operation: &'static str,
        operand1: impl Into<OsString>,
        source: io::Error,
    ) -> Self {
        Self::Io1 {
            operation,
            operand1: operand1.into(),
            source,
        }
    }

    pub(crate) fn from_selinux(operation: &'static str, source: selinux::errors::Error) -> Self {
        Self::SELinux { operation, source }
    }
}

pub(crate) fn write_full_error<W>(writer: &mut W, err: &dyn std::error::Error) -> std::fmt::Result
where
    W: Write,
{
    write!(writer, "{err}")?;
    let mut err = err;
    while let Some(source) = err.source() {
        err = source;
        write!(writer, ": {err}")?;
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct RunconError {
    inner: Error,
    code: i32,
}

impl RunconError {
    pub(crate) fn new(e: Error) -> Self {
        Self::with_code(error_exit_status::ANOTHER_ERROR, e)
    }

    pub(crate) fn with_code(code: i32, e: Error) -> Self {
        Self { inner: e, code }
    }
}

impl std::error::Error for RunconError {}
impl UError for RunconError {
    fn code(&self) -> i32 {
        self.code
    }
}
impl Display for RunconError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write_full_error(f, &self.inner)
    }
}

impl UError for Error {
    fn code(&self) -> i32 {
        match self {
            Self::MissingCommand => error_exit_status::ANOTHER_ERROR,
            Self::SELinuxNotEnabled => error_exit_status::ANOTHER_ERROR,
            Self::NotUTF8(_) => error_exit_status::ANOTHER_ERROR,
            Self::CommandLine(e) => e.exit_code(),
            Self::SELinux { .. } => error_exit_status::ANOTHER_ERROR,
            Self::Io { .. } => error_exit_status::ANOTHER_ERROR,
            Self::Io1 { .. } => error_exit_status::ANOTHER_ERROR,
        }
    }
}
