use failure::*;
use reqwest::{Error as ReqwestError};
use reqwest::header::{InvalidHeaderValue, ToStrError as ReqwestToStrError};
use std::fmt;
use std::io::{Error as IoError};
use std::num::ParseIntError;
use std::str::ParseBoolError;
use websocket::result::WebSocketError;

pub(crate) use std::result::{Result as StdResult};

#[derive(Fail, Debug)]
pub enum ErrorKind {
    #[fail(display = "Bot token is not valid: {}", _0)]
    InvalidBotToken(&'static str),
    #[fail(display = "Discord returned bad response: {}", _0)]
    DiscordBadResponse(&'static str),
    #[fail(display = "Internal error: {}", _0)]
    InternalError(&'static str),

    #[fail(display = "An IO error occurred: {}", _0)]
    IoError(#[cause] IoError),
    #[fail(display = "{}", _0)]
    ParseBoolError(#[cause] std::str::ParseBoolError),
    #[fail(display = "{}", _0)]
    ParseIntError(#[cause] std::num::ParseIntError),
    #[fail(display = "Error making HTTP request: {}", _0)]
    ReqwestError(#[cause] ReqwestError),
    #[fail(display = "Could not convert value to HTTP header: {}", _0)]
    ReqwestHeaderError(#[cause] InvalidHeaderValue),
    #[fail(display = "Could not convert HTTP header to string: {}", _0)]
    ReqwestToStrError(#[cause] ReqwestToStrError),
    #[fail(display = "Websocket error: {}", _0)]
    WebSocketError(#[cause] WebSocketError),
}

struct ErrorData {
    kind: ErrorKind, backtrace: Option<Backtrace>, cause: Option<Box<dyn Fail>>,
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
        Self::new(kind).with_backtrace()
    }

    #[inline(never)] #[cold]
    pub fn with_backtrace(mut self) -> Self {
        self.0.backtrace = Some(Backtrace::new());
        self
    }

    fn with_cause(mut self, cause: Box<dyn Fail>) -> Self {
        self.0.cause = Some(cause);
        self
    }

    pub fn error_kind(&self) -> &ErrorKind {
        &self.0.kind
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0.kind, f)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    IoError => IoError,
    ParseBoolError => ParseBoolError,
    ParseIntError => ParseIntError,
    ReqwestError => ReqwestError,
    ReqwestHeaderError => InvalidHeaderValue,
    ReqwestToStrError => ReqwestToStrError,
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
