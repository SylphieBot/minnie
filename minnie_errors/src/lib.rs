#![deny(unused_must_use)]

//! Defines the error type used by Minnie.

use backtrace::Backtrace;
use futures::FutureExt;
use std::any::Any;
use std::borrow::Cow;
use std::error::{Error as StdError};
use std::fmt;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use thiserror::*;

pub use std::result::{Result as StdResult};

mod status;
pub use status::{DiscordError, DiscordErrorCode};

#[doc(inline)]
pub use http::{StatusCode as HttpStatusCode};

#[derive(Debug)]
pub struct LibError(Box<dyn StdError + Send + 'static>);
impl <T: StdError + Send + 'static> From<T> for LibError {
    #[inline(never)] #[cold]
    fn from(t: T) -> Self {
        LibError(Box::new(t))
    }
}

/// Represents the kind of error that occurred.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Invalid input was provided to the library.
    ///
    /// This generally indicates a bug in an user of the library.
    #[error("Invalid API usage: {0}")]
    InvalidInput(&'static str),
    /// An IO error occurred.
    ///
    /// This generally occurs because Discord is experiencing issues.
    #[error("IO Error: {0}")]
    IoError(&'static str),
    /// An internal error has occurred.
    ///
    /// This generally indicates a bug in the library.
    #[error("Internal error: {0}")]
    InternalError(&'static str),
    /// Used to convey information about a panic to the gateway or voice event receivers.
    ///
    /// This should not be returned from other methods in normal circumstances, and panics in
    /// most library code will directly propagate to the caller.
    #[error("{0}")]
    Panicked(Cow<'static, str>),

    /// Discord returned an unexpected or invalid response.
    ///
    /// This may happen if Discord is experiencing issues or the library hasn't been updated
    /// for a change in Discord's protocol.
    #[error("Discord returned bad response: {0}")]
    DiscordBadResponse(&'static str),
    /// Discord returned an unexpected or invalid response.
    ///
    /// This may happen if Discord is experiencing issues or the library hasn't been updated
    /// for a change in Discord's protocol.
    #[error("Discord returned unparsable packet: {0:?}")]
    DiscordUnparsablePacket(String),
    /// Discord returned an error status code.
    #[error("{0} failed with {1} ({2})")]
    RequestFailed(&'static str, HttpStatusCode, DiscordError),
}

struct ErrorData {
    kind: ErrorKind,
    backtrace: Option<Backtrace>,
    cause: Option<LibError>,
}

/// An error type used throughout the library.
pub struct Error(Box<ErrorData>);
impl Error {
    #[inline(never)] #[cold]
    pub fn new(kind: ErrorKind) -> Self {
        Error(Box::new(ErrorData {
            kind, backtrace: None, cause: None,
        }))
    }

    #[inline(never)] #[cold]
    pub fn new_with_cause(kind: ErrorKind, cause: LibError) -> Self {
        let mut err = Error::new(kind);
        err.0.cause = Some(cause);
        err
    }

    #[inline(never)] #[cold]
    pub fn new_with_backtrace(kind: ErrorKind) -> Self {
        Error::new(kind).with_backtrace()
    }

    fn with_backtrace(mut self) -> Self {
        if !self.backtrace().is_some() {
            self.0.backtrace = Some(Backtrace::new());
        }
        self
    }

    #[inline(never)] #[cold]
    fn wrap_panic(panic: Box<dyn Any + Send + 'static>) -> Error {
        let panic: Cow<'static, str> = if let Some(s) = panic.downcast_ref::<&'static str>() {
            (*s).into()
        } else if let Some(s) = panic.downcast_ref::<String>() {
            s.clone().into()
        } else {
            "<non-string panic info>".into()
        };
        Error::new(ErrorKind::Panicked(panic))
    }

    /// Returns the type of error contained in this object.
    pub fn error_kind(&self) -> &ErrorKind {
        &self.0.kind
    }

    /// Returns the backtrace, if one was recorded.
    pub fn backtrace(&self) -> Option<&Backtrace> {
        self.0.backtrace.as_ref()
    }

    /// Returns `true` if this error was likely due to a bug in either user code or Minnie.
    pub fn is_error(&self) -> bool {
        match self.error_kind() {
            ErrorKind::InternalError(_) | ErrorKind::InvalidInput(_) | ErrorKind::Panicked(_) =>
                true,
            _ => false,
        }
    }

    /// Returns `true` if this error was due to an IO or network problem.
    pub fn is_io(&self) -> bool {
        match self.error_kind() {
            ErrorKind::IoError(_) => true,
            _ => false,
        }
    }

    /// Returns `true` if this error originated from Discord.
    pub fn is_discord(&self) ->  bool {
        match self.error_kind() {
            ErrorKind::DiscordBadResponse(_) | ErrorKind::RequestFailed(_, _, _) => true,
            _ => false,
        }
    }
}
impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self.0.cause.as_ref() {
            Some(x) => Some(&*x.0),
            None => None,
        }
    }
}
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Error")
            .field(&self.0.kind)
            .field(&self.0.cause)
            .finish()
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0.kind, f)?;
        Ok(())
    }
}

/// The result type used throughout the library.
pub type Result<T> = StdResult<T, Error>;

pub type LibResult<T> = StdResult<T, LibError>;

// Helpers for error handling
pub trait ErrorExt<T>: Sized {
    fn context(self, kind: ErrorKind) -> Result<T>;

    fn io_err(self, text: &'static str) -> Result<T> {
        self.context(ErrorKind::IoError(text))
    }
    fn bad_response(self, text: &'static str) -> Result<T> {
        self.context(ErrorKind::DiscordBadResponse(text))
    }
    fn internal_err(self, text: &'static str) -> Result<T> {
        self.context(ErrorKind::InternalError(text))
    }
    fn invalid_input(self, text: &'static str) -> Result<T> {
        self.context(ErrorKind::InvalidInput(text))
    }

    fn unexpected(self) -> Result<T> {
        self.internal_err("Unexpected error encountered.")
    }
}
impl <T> ErrorExt<T> for Option<T> {
    #[inline(always)]
    fn context(self, kind: ErrorKind) -> Result<T> {
        match self {
            Some(x) => Ok(x),
            None => Err(Error::new_with_backtrace(kind)),
        }
    }
}
impl <T, E: Into<LibError>> ErrorExt<T> for StdResult<T, E> {
    #[inline(always)]
    fn context(self, kind: ErrorKind) -> Result<T> {
        match self {
            Ok(x) => Ok(x),
            Err(e) => Err(Error::new_with_cause(kind, e.into())),
        }
    }
}

pub fn catch_panic<T>(func: impl FnOnce() -> Result<T>) -> Result<T> {
    match catch_unwind(AssertUnwindSafe(func)) {
        Ok(r) => r,
        Err(e) => Err(Error::wrap_panic(e)),
    }
}

pub async fn catch_panic_async<T>(fut: impl Future<Output = Result<T>>) -> Result<T> {
    match AssertUnwindSafe(fut).catch_unwind().await {
        Ok(v) => v,
        Err(panic) => Err(Error::wrap_panic(panic)),
    }
}

#[macro_export]
macro_rules! error_kind {
    ($error:literal $(,)?) => {
        crate::errors::ErrorKind::InternalError($error)
    };
    ($variant:ident, $($body:expr),* $(,)?) => {
        crate::errors::ErrorKind::$variant($($body,)*)
    };
}

#[macro_export]
macro_rules! bail {
    ($($tt:tt)*) => {
        return Err(crate::errors::Error::new_with_backtrace(error_kind!($($tt)*)))
    }
}

#[macro_export]
macro_rules! ensure {
    ($check:expr, $($tt:tt)*) => {
        if !$check {
            bail!($($tt)*);
        }
    }
}
