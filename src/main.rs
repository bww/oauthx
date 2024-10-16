use std::process;

use tokio;
use tokio::sync::mpsc;
use futures_util::future::TryFutureExt;

use clap::Parser;
use warp::Filter;
use colored::Colorize;

mod error;
mod oauth2;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Options {
  #[clap(long, help="Enable debugging mode")]
  pub debug: bool,
  #[clap(long, help="Enable verbose output")]
  pub verbose: bool,
  #[clap(long="client:id", help="The OAuth2 consumer client identifier")]
  pub client_id: Option<String>,
  #[clap(long="client:secret", help="The OAuth2 consumer client secret")]
  pub client_secret: Option<String>,
  #[clap(long="url:auth", help="The OAuth2 service authorization URL")]
  pub auth_url: Option<String>,
  #[clap(long="url:token", help="The OAuth2 service token URL")]
  pub token_url: Option<String>,
  #[clap(long="url:return", help="The OAuth2 consumer callback URL")]
  pub return_url: Option<String>,
  #[clap(long="config", help="The OAuth2 consumer configuration")]
  pub config: Option<String>,
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
  let conf = match &opts.config {
    Some(p) => oauth2::Consumer::read(p)?,
    None    => oauth2::Consumer::empty(),
  };

  let (tx, mut rx) = mpsc::channel(1);
  let tx_filter = warp::any().map(move || tx.clone());

  let routes = warp::path("return")
    .and(tx_filter)
    .map(|tx: mpsc::Sender<()>| {
      let _ = tx.send(());
      "Ok"
    });

  let (_, server) = warp::serve(routes)
    .bind_with_graceful_shutdown(([127, 0, 0, 1], 3030), async move {
      rx.recv().await;
    });

  let _ = tokio::task::spawn(server).await;
  Ok(0)
}
