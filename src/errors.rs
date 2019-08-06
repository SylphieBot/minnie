use failure::*;
use flate2::DecompressError;
use futures::FutureExt;
use parking_lot::Mutex;
use reqwest::{Error as ReqwestError};
use reqwest::header::{InvalidHeaderValue, ToStrError as ReqwestToStrError};
use serde_json::{Error as SerdeJsonError};
use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use std::future::Future;
use std::io::{Error as IoError};
use std::num::ParseIntError;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::str::ParseBoolError;
use websocket::{WebSocketError, CloseData};

pub(crate) use std::result::{Result as StdResult};

pub struct PanicWrapper(pub Mutex<Box<dyn Any + Send + 'static>>);
impl PanicWrapper {
    pub fn as_str(&self) -> String {
        let lock = self.0.lock();
        if let Some(s) = (*lock).downcast_ref::<&'static str>() {
            (*s).into()
        } else if let Some(s) = (*lock).downcast_ref::<String>() {
            s.clone().into()
        } else {
            "<non-string panic info>".into()
        }
    }
}
impl fmt::Debug for PanicWrapper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("PanicWrapper").field(&self.as_str()).finish()
    }
}
impl fmt::Display for PanicWrapper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.as_str())
    }
}

#[derive(Fail, Debug)]
pub enum ErrorKind {
    #[fail(display = "Bot token is not valid: {}", _0)]
    InvalidBotToken(&'static str),
    #[fail(display = "Discord returned bad response: {}", _0)]
    DiscordBadResponse(&'static str),
    #[fail(display = "Internal error: {}", _0)]
    InternalError(&'static str),
    #[fail(display = "{}", _0)]
    Panicked(PanicWrapper),

    #[fail(display = "Websocket disconnected: {:?}", _0)]
    WebsocketDisconnected(Option<CloseData>),
    #[fail(display = "Failed to parse websocket packet.")]
    PacketParseError,

    // Wrappers for external error types.
    #[fail(display = "Error decompressing a packet")]
    DecompressError(#[cause] DecompressError),
    #[fail(display = "An IO error occurred")]
    IoError(#[cause] IoError),
    #[fail(display = "Error parsing boolean")]
    ParseBoolError(std::str::ParseBoolError),
    #[fail(display = "Error parsing integer: {}", _0)]
    ParseIntError(std::num::ParseIntError),
    #[fail(display = "Error making HTTP request")]
    ReqwestError(#[cause] ReqwestError),
    #[fail(display = "Could not convert value to HTTP header")]
    ReqwestHeaderError(#[cause] InvalidHeaderValue),
    #[fail(display = "Could not convert HTTP header to string")]
    ReqwestToStrError(#[cause] ReqwestToStrError),
    #[fail(display = "Error parsing JSON")]
    SerdeJsonError(#[cause] SerdeJsonError),
    #[fail(display = "Websocket error")]
    WebSocketError(#[cause] WebSocketError),
}

pub struct ErrorData {
    pub kind: ErrorKind,
    pub backtrace: Option<Backtrace>,
    pub cause: Option<Box<dyn Fail>>,
}

pub(crate) fn find_backtrace(fail: &impl Fail) -> Option<&Backtrace> {
    let mut current: Option<&dyn Fail> = Some(fail);
    while let Some(x) = current {
        if let Some(bt) = x.backtrace() {
            return Some(bt)
        }
        current = x.cause();
    }
    None
}

pub struct Error(Box<ErrorData>);
impl Error {
    #[inline(never)] #[cold]
    pub fn new(kind: ErrorKind) -> Self {
        Error(Box::new(ErrorData {
            kind, backtrace: None, cause: None
        }))
    }

    #[inline(never)] #[cold]
    pub fn new_with_cause(kind: ErrorKind, cause: impl Fail) -> Self {
        Error::new(kind).with_cause(Box::new(cause))
    }

    #[inline(never)] #[cold]
    pub fn new_with_backtrace(kind: ErrorKind) -> Self {
        Error::new(kind).with_backtrace()
    }

    #[inline(never)] #[cold]
    pub fn with_backtrace(mut self) -> Self {
        self.0.backtrace = Some(Backtrace::new());
        self
    }

    #[inline(never)] #[cold]
    fn wrap_panic(panic: Box<dyn Any + Send + 'static>) -> Error {
        Error::new(ErrorKind::Panicked(PanicWrapper(Mutex::new(panic))))
    }

    /// Catches panics that occur in a closure, wrapping them in an [`Error`].
    pub fn catch_panic<T>(func: impl FnOnce() -> Result<T>) -> Result<T> {
        match catch_unwind(AssertUnwindSafe(func)) {
            Ok(r) => r,
            Err(e) => Err(Error::wrap_panic(e)),
        }
    }

    /// Catches panics that occur in a future, wrapping then in an [`Error`].
    pub async fn catch_panic_async<T>(fut: impl Future<Output = Result<T>>) -> Result<T> {
        match AssertUnwindSafe(fut).catch_unwind().await {
            Ok(v) => v,
            Err(panic) => Err(Error::wrap_panic(panic)),
        }
    }

    fn with_cause(mut self, cause: Box<dyn Fail>) -> Self {
        self.0.cause = Some(cause);
        self
    }

    pub fn into_inner(self) -> ErrorData {
        *self.0
    }

    pub fn error_kind(&self) -> &ErrorKind {
        &self.0.kind
    }

    /// Finds the first backtrace in the cause chain.
    pub fn find_backtrace(&self) -> Option<&Backtrace> {
        find_backtrace(self)
    }
}
impl Fail for Error {
    fn name(&self) -> Option<&str> {
        Some("minnie::errors::Error")
    }

    fn cause(&self) -> Option<&dyn Fail> {
        match self.0.kind.cause() {
            Some(x) => Some(x),
            None => self.0.cause.as_ref().map(|x| &**x),
        }
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.0.backtrace.as_ref()
    }
}
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0.kind, f)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0.kind, f)
    }
}

pub type Result<T> = StdResult<T, Error>;

macro_rules! generic_from {
    ($($branch:ident => $ty:ty),* $(,)?) => {$(
        impl From<$ty> for Error {
            #[inline(never)] #[cold]
            fn from(err: $ty) -> Self {
                Error::new(ErrorKind::$branch(err)).with_backtrace()
            }
        }
    )*}
}
generic_from! {
    DecompressError => DecompressError,
    IoError => IoError,
    ParseBoolError => ParseBoolError,
    ParseIntError => ParseIntError,
    ReqwestError => ReqwestError,
    ReqwestHeaderError => InvalidHeaderValue,
    ReqwestToStrError => ReqwestToStrError,
    SerdeJsonError => SerdeJsonError,
    WebSocketError => WebSocketError,
}

// Helpers for error handling
pub(crate) trait ErrorExt<T> {
    fn context(self, kind: ErrorKind) -> Result<T>;
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
impl <T, E: Into<Error>> ErrorExt<T> for StdResult<T, E> {
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
