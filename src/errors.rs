//! Defines the error types used by Minnie.

use crate::http::{DiscordError, HttpStatusCode};
use failure::*;
use flate2::DecompressError;
use futures::FutureExt;
use reqwest::{Error as ReqwestError};
use reqwest::header::{InvalidHeaderValue, ToStrError as ReqwestToStrError};
use serde_json::{Error as SerdeJsonError};
use std::any::Any;
use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt;
use std::future::Future;
use std::io::{Error as IoError};
use std::num::{ParseIntError, ParseFloatError};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::str::ParseBoolError;
use webpki::InvalidDNSNameError;
use websocket::WebSocketError;

pub use std::result::{Result as StdResult};


macro_rules! lib_error {
    ($($ty:ident),* $(,)?) => {
        #[derive(Fail, Debug)]
        pub enum LibError {$(
            #[fail(display = "{}", _0)]
            $ty(#[cause] $ty),
        )*}
        $(
            impl From<$ty> for LibError {
                #[inline(never)] #[cold]
                fn from(err: $ty) -> Self {
                    LibError::$ty(err)
                }
            }
        )*
    }
}
lib_error! {
    DecompressError, InvalidDNSNameError, IoError, ParseBoolError, ParseIntError, ParseFloatError,
    ReqwestError, InvalidHeaderValue, ReqwestToStrError, SerdeJsonError, WebSocketError,
}
impl From<Infallible> for LibError {
    fn from(_: Infallible) -> Self {
        panic!("wtf")
    }
}

/// Represents the kind of error that occurred.
#[derive(Fail, Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Invalid input was provided to the library.
    ///
    /// This generally indicates a bug in an user of the library.
    #[fail(display = "Invalid API usage: {}", _0)]
    InvalidInput(&'static str),
    /// An IO error occurred.
    ///
    /// This generally occurs because Discord is experiencing issues.
    #[fail(display = "IO Error: {}", _0)]
    IoError(&'static str),
    /// An internal error has occurred.
    ///
    /// This generally indicates a bug in the library.
    #[fail(display = "Internal error: {}", _0)]
    InternalError(&'static str),
    /// Used to convey information about a panic to the gateway or voice event receivers.
    ///
    /// This should not be returned from other methods in normal circumstances, and panics in
    /// most library code will directly propagate to the caller.
    #[fail(display = "{}", _0)]
    Panicked(Cow<'static, str>),

    /// Discord returned an unexpected or invalid response.
    ///
    /// This may happen if Discord is experiencing issues or the library hasn't been updated
    /// for a change in Discord's protocol.
    #[fail(display = "Discord returned bad response: {}", _0)]
    DiscordBadResponse(&'static str),
    /// Discord returned an error status code.
    #[fail(display = "{} failed with {} ({})", _0, _1, _2)]
    RequestFailed(&'static str, HttpStatusCode, DiscordError),
}

struct ErrorData {
    kind: ErrorKind,
    backtrace: Option<Backtrace>,
    cause: Option<LibError>,
}

pub fn find_backtrace(fail: &dyn Fail) -> Option<&Backtrace> {
    let mut current: Option<&dyn Fail> = Some(&*fail);
    while let Some(x) = current {
        if let Some(bt) = x.backtrace() {
            return Some(bt)
        }
        current = x.cause();
    }
    None
}

/// An error type used throughout the library.
pub struct Error(Box<ErrorData>);
impl Error {
    #[inline(never)] #[cold]
    fn new(kind: ErrorKind) -> Self {
        Error(Box::new(ErrorData {
            kind, backtrace: None, cause: None,
        }))
    }

    #[inline(never)] #[cold]
    pub(crate) fn new_with_cause(kind: ErrorKind, cause: LibError) -> Self {
        let mut err = Error::new(kind);
        err.0.cause = Some(cause);
        err
    }

    #[inline(never)] #[cold]
    pub(crate) fn new_with_backtrace(kind: ErrorKind) -> Self {
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

    pub(crate) fn catch_panic<T>(func: impl FnOnce() -> Result<T>) -> Result<T> {
        match catch_unwind(AssertUnwindSafe(func)) {
            Ok(r) => r,
            Err(e) => Err(Error::wrap_panic(e)),
        }
    }

    pub(crate) async fn catch_panic_async<T>(fut: impl Future<Output = Result<T>>) -> Result<T> {
        match AssertUnwindSafe(fut).catch_unwind().await {
            Ok(v) => v,
            Err(panic) => Err(Error::wrap_panic(panic)),
        }
    }

    /// Returns the type of error contained in this object.
    pub fn error_kind(&self) -> &ErrorKind {
        &self.0.kind
    }

    /// Finds the first backtrace in the cause chain.
    pub fn find_backtrace(&self) -> Option<&Backtrace> {
        find_backtrace(self)
    }

    // TODO: Add is_* helpers?
}
impl Fail for Error {
    fn name(&self) -> Option<&str> {
        Some("minnie::errors::Error")
    }

    fn cause(&self) -> Option<&dyn Fail> {
        self.0.cause.as_ref().and_then(|x| x.cause())
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.0.backtrace.as_ref()
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
        if let Some(x) = &self.0.cause {
            f.write_str(" (caused by: ")?;
            fmt::Display::fmt(x, f)?;
            f.write_str(")")?;
        }
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

macro_rules! error_kind {
    ($error:literal $(,)?) => {
        crate::errors::ErrorKind::InternalError($error)
    };
    ($variant:ident, $($body:expr),* $(,)?) => {
        crate::errors::ErrorKind::$variant($($body,)*)
    };
}
macro_rules! bail {
    ($($tt:tt)*) => {
        return Err(crate::errors::Error::new_with_backtrace(error_kind!($($tt)*)))
    }
}
macro_rules! ensure {
    ($check:expr, $($tt:tt)*) => {
        if !$check {
            bail!($($tt)*);
        }
    }
}
