use std::process;
use colored::Colorize;

use tokio;
use clap::Parser;
use warp::Filter;

mod error;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Options {
  #[clap(long, help="Enable debugging mode")]
  pub debug: bool,
  #[clap(long, help="Enable verbose output")]
  pub verbose: bool,
  #[clap(long="client:id", help="The OAuth2 consumer client identifier")]
  pub client_id: String,
  #[clap(long="client:secret", help="The OAuth2 consumer client secret")]
  pub client_secret: String,
  #[clap(long="url:auth", help="The OAuth2 service authorization URL")]
  pub auth_url: String,
  #[clap(long="url:token", help="The OAuth2 service token URL")]
  pub token_url: String,
  #[clap(long="url:return", help="The OAuth2 consumer callback URL")]
  pub return_url: String,
}

#[tokio::main]
async fn main() {
  match cmd().await {
    Ok(code)  => process::exit(code),
    Err(err)  => {
      eprintln!("{}", &format!("* * * {}", err).yellow().bold());
      process::exit(1);
    },
  };
}

async fn cmd() -> Result<i32, error::Error> {
  let opts = Options::parse();
  println!("HI {} {}", &opts.auth_url, &opts.token_url);
  let birthday = warp::path!("birthday")
    .map(|| {
      "Ok"
    });
  warp::serve(birthday)
    .run(([127, 0, 0, 1], 3030))
    .await;
  Ok(0)
}
