use std::io;
use std::fmt;

use url;

use crate::oauth2;

#[derive(Debug)]
pub enum Error {
  Message(String),
  IOError(io::Error),
  UrlParseError(url::ParseError),
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

impl From<url::ParseError> for Error {
  fn from(err: url::ParseError) -> Self {
    Self::UrlParseError(err)
  }
}

impl From<oauth2::error::Error> for Error {
  fn from(err: oauth2::error::Error) -> Self {
    Self::OAuth2Error(err)
  }
}

impl warp::reject::Reject for Error {}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Message(msg)       => write!(f, "{}", msg),
      Self::IOError(err)       => err.fmt(f),
      Self::UrlParseError(err) => err.fmt(f),
      Self::OAuth2Error(err)   => err.fmt(f),
    }
  }
}
