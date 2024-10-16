use std::process;

use tokio;
use tokio::sync::oneshot;
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

  let r_url = match conf.return_url.or(opts.return_url) {
    Some(r_url) => r_url,
    None        => return Err(error::Error::new("Invalid configuration")),
  };

  let (tx, rx) = oneshot::channel();
  let routes = warp::any()
    .map(|| {
      "Ok"
    });
  let (addr, server) = warp::serve(routes)
    .bind_with_graceful_shutdown(([127, 0, 0, 1], 3030), async {
      rx.await.ok();
    });

  let _ = tx.send(());
  Ok(0)
}
