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
  let birthday = warp::path!("birthday")
    .map(|| {
      "Ok"
    });
  warp::serve(birthday)
    .run(([127, 0, 0, 1], 3030))
    .await;
  Ok(0)
}
