use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io;

use nix::Error as NixError;
use zip::result::ZipError;

type PestError = pest::error::Error<crate::parser::Rule>;

#[derive(Debug)]
pub struct ThreadError(pub String);

impl ToString for ThreadError {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

#[derive(Debug)]
pub enum GenericError {
    IOError(io::Error),
    ParseError(PestError),
    ZipError(ZipError),
    NixError(NixError),
    NetworkError(curl::Error),
    JsonError(serde_json::Error),
    ThreadError(ThreadError),
    Other(Box<dyn Error>),
}

impl Display for GenericError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let message = match self {
            GenericError::IOError(err) => format!("IOError: {}", err.to_string()),
            GenericError::ParseError(err) => format!("ParseError: {}", err.to_string()),
            GenericError::ZipError(err) => format!("ZipError: {}", err.to_string()),
            GenericError::NixError(err) => format!("NixError: {}", err.to_string()),
            GenericError::JsonError(err) => format!("JsonError: {}", err.to_string()),
            GenericError::NetworkError(err) => format!("NetworkError: {}", err.to_string()),
            GenericError::ThreadError(err) => format!("ThreadError: {}", err.to_string()),
            GenericError::Other(err) => format!("OtherError: {}", err.to_string()),
        };
        write!(f, "{}", message)
    }
}

impl Error for GenericError {}

macro_rules! error_from {
    ($type_target:ty, $type:ty, $enum_ident:expr) => {
        impl From<$type> for $type_target {
            fn from(err: $type) -> Self {
                $enum_ident(err)
            }
        }
    }
}

error_from!(GenericError, io::Error, GenericError::IOError);
error_from!(GenericError, PestError, GenericError::ParseError);
error_from!(GenericError, ZipError, GenericError::ZipError);
error_from!(GenericError, curl::Error, GenericError::NetworkError);
error_from!(GenericError, nix::Error, GenericError::NixError);
error_from!(GenericError, serde_json::Error, GenericError::JsonError);

