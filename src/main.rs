use std::io;
use std::io::Write;
use std::process;
use std::collections::HashMap;

use tokio;
use tokio::sync::mpsc;

use clap::Parser;
use warp::Filter;
use colored::Colorize;
use url::Url;
use rand::distributions::{Alphanumeric, DistString};
use handlebars;
use serde_json::json;
use open;

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
  #[clap(long="state", help="The state to use to validate the response; if no state is provided, random state will be used")]
  pub state: Option<String>,
  #[clap(long="config", help="The OAuth2 consumer configuration")]
  pub config: Option<String>,
  #[clap(long="passive", help="Wait for a response but do not initiate the OAuth2 flow")]
  pub passive: bool,
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

  let state = match opts.state {
    Some(val) => val.to_string(),
    None      => Alphanumeric.sample_string(&mut rand::thread_rng(), 64),
  };
  let auth_url = match opts.auth_url.or(conf.auth_url) {
    Some(url) => url,
    None      => return Err(error::Error::new("No auth URL defined in configuration")),
  };
  let client_id = match opts.client_id.or(conf.client_id) {
    Some(val) => val,
    None      => return Err(error::Error::new("No client ID defined in configuration")),
  };

  let mut url = Url::parse(&auth_url)?;
  url.query_pairs_mut()
    .append_pair("response_type", "code")
    .append_pair("state", &state)
    .append_pair("client_id", &client_id)
    .finish();

  let (tx, mut rx) = mpsc::channel(1);
  let tx_filter = warp::any().map(move || tx.clone());
  let routes = warp::path("return")
    .and(tx_filter)
    .and(warp::query::<HashMap<String, String>>())
    .and_then(|tx: mpsc::Sender<()>, query: HashMap<String, String>| async move {
      println!(">>> {:?}", query);
      let reg = handlebars::Handlebars::new();
      let rsp = match reg.render_template(TEMPLATE_RESPONSE, &json!(query)) {
        Ok(rsp) => rsp,
        Err(_)  => return Err(warp::reject::reject()),
      };
      match tx.send(()).await {
        Ok(_)    => Ok(warp::reply::html(rsp)),
        Err(err) => {
          println!("Could not cancel: {}", err);
          Err(warp::reject::reject())
        },
      }
    });

  let (_, server) = warp::serve(routes)
    .bind_with_graceful_shutdown(([127, 0, 0, 1], 4000), async move {
      rx.recv().await;
    });

  if !opts.passive {
    println!("\nOpening the OAuth2 flow in your browser:\n    ➤ {}\n\nIf your browser doesn't open; paste the link in manually...", url.as_str());
    confirm("\nInitiate the OAuth2 flow? [y/N] ", "y")?;
    open::that(url.as_str())?;
  }

  println!("Waiting for a response from the service...");
  let _ = tokio::task::spawn(server).await;
  Ok(0)
}

fn confirm(prompt: &str, expect: &str) -> Result<(), error::Error> {
  print!("{}", prompt);
  io::stdout().flush()?;
  let mut rsp = String::new();
  if let Err(err) = io::stdin().read_line(&mut rsp) {
    Err(err.into())
  } else if rsp.to_lowercase().eq(expect) {
    Err(error::Error::new("Canceled"))
  }else{
    Ok(())
  }
}

const TEMPLATE_RESPONSE: &str = r#"<html>
<head>
  <style>
    td.key {
      font-weight: 700;
      text-align: right;
    }
  </style>
</head>
<body>
  <h1>Connection created</h1>
  <p>Received the following data in the response</p>
  <table>
  {{ #each this }}
    <tr>
      <td class="key"><code>{{ @key }}</code></td>
      <td><code>{{ this }}</code></td>
    </tr>
  {{ /each }}
  </table>
</body>
</html>
"#;
