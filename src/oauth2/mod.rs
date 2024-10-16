use std::fs;

use serde::{Serialize, Deserialize};

pub mod error;

#[derive(Serialize, Deserialize)]
pub struct Consumer {
  #[serde(rename(serialize="client-id", deserialize="client-id"))]
  pub client_id: Option<String>,
  #[serde(rename(serialize="client-secret", deserialize="client-secret"))]
  pub client_secret: Option<String>,
  #[serde(rename(serialize="auth-url", deserialize="auth-url"))]
  pub auth_url: Option<String>,
  #[serde(rename(serialize="token-url", deserialize="token-url"))]
  pub token_url: Option<String>,
  #[serde(rename(serialize="return-url", deserialize="return-url"))]
  pub return_url: Option<String>,
}

impl Consumer {
  pub fn empty() -> Consumer {
    Consumer{
      client_id: None,
      client_secret: None,
      auth_url: None,
      token_url: None,
      return_url: None,
    }
  }

  pub fn read(path: &str) -> Result<Consumer, error::Error> {
    let data = fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&data)?)
  }
}
