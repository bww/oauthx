use std::io;
use std::fmt;

use serde_yaml;

#[derive(Debug)]
pub enum Error {
  IOError(io::Error),
  YamlError(serde_yaml::Error),
}

impl From<io::Error> for Error {
  fn from(err: io::Error) -> Self {
    Self::IOError(err)
  }
}

impl From<serde_yaml::Error> for Error {
  fn from(err: serde_yaml::Error) -> Self {
    Self::YamlError(err)
  }
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::IOError(err)   => err.fmt(f),
      Self::YamlError(err) => err.fmt(f),
    }
  }
}
