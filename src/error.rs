use std::io;
use std::fmt;

use crate::oauth2;

#[derive(Debug)]
pub enum Error {
  Message(String),
  IOError(io::Error),
  OAuth2Error(oauth2::error::Error),
}

impl Error {
  pub fn new(msg: &str) -> Error {
    Self::Message(msg.to_owned())
  }
}

impl From<io::Error> for Error {
  fn from(err: io::Error) -> Self {
    Self::IOError(err)
  }
}

impl From<oauth2::error::Error> for Error {
  fn from(err: oauth2::error::Error) -> Self {
    Self::OAuth2Error(err)
  }
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Message(msg)     => write!(f, "{}", msg),
      Self::IOError(err)     => err.fmt(f),
      Self::OAuth2Error(err) => err.fmt(f),
    }
  }
}
