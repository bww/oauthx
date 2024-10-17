use std::io;
use std::io::Write;
use std::process;
use std::collections::HashMap;

use serde::Serialize;
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
  #[clap(long="grant:type", help="The OAuth2 grant type to request")]
  pub grant_type: Option<String>,
  #[clap(long="scopes", help="The scopes we are requesting")]
  pub scopes: Vec<String>,
  #[clap(long="state", help="The state to use to validate the response; if no state is provided, random state will be used")]
  pub state: Option<String>,
  #[clap(long="config", help="The OAuth2 consumer configuration")]
  pub config: Option<String>,
  #[clap(long="passive", help="Wait for a response but do not initiate the OAuth2 flow")]
  pub passive: bool,
  #[clap(long="interactive", help="Prompt for interactive confirmation")]
  pub interactive: bool,
}

impl Options {
  fn merge_config(&self, conf: &oauth2::Consumer) -> oauth2::Consumer {
    oauth2::Consumer{
      client_id: self.client_id.clone().or(conf.client_id.clone()),
      client_secret: self.client_secret.clone().or(conf.client_secret.clone()),
      auth_url: self.auth_url.clone().or(conf.auth_url.clone()),
      token_url: self.token_url.clone().or(conf.token_url.clone()),
      return_url: self.return_url.clone().or(conf.return_url.clone()),
      grant_type: self.grant_type.clone().or(conf.grant_type.clone()),
      scopes: match &self.scopes.len() {
        0 => conf.scopes.clone(),
        _ => self.scopes.clone(),
      },
    }
  }
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

  let rspconf = opts.clone().merge_config(&conf);
  if rspconf.scopes.len() == 0 {
    return Err(error::Error::new("No scopes are requested"));
  }
  let auth_url = match opts.auth_url.or(conf.auth_url) {
    Some(url) => url,
    None      => return Err(error::Error::new("No auth URL defined in configuration")),
  };
  let client_id = match opts.client_id.or(conf.client_id) {
    Some(val) => val,
    None      => return Err(error::Error::new("No client ID defined in configuration")),
  };
  let client_secret = match opts.client_secret.or(conf.client_secret) {
    Some(val) => val,
    None      => return Err(error::Error::new("No client secret defined in configuration")),
  };
  let state = match opts.state {
    Some(val) => val.to_string(),
    None      => Alphanumeric.sample_string(&mut rand::thread_rng(), 64),
  };

  let mut url = Url::parse(&auth_url)?;
  url.query_pairs_mut()
    .append_pair("response_type", "code")
    .append_pair("state", &state)
    .append_pair("scope", &conf.scopes.join(" "))
    .append_pair("client_id", &client_id)
    .finish();

  let (tx, mut rx) = mpsc::channel(1);
  let tx_filter = warp::any().map(move || tx.clone());
  let config_filter = warp::any().map(move || rspconf.clone());
  let client_filter = warp::any().map(move || (client_id.clone(), client_secret.clone()));
  let state_filter = warp::any().map(move || state.clone());
  let routes = warp::path("return")
    .and(tx_filter)
    .and(config_filter)
    .and(client_filter)
    .and(state_filter)
    .and(warp::query::<HashMap<String, String>>())
    .and_then(|tx: mpsc::Sender<()>, conf: oauth2::Consumer, client: (String, String), state: String, query: HashMap<String, String>| async move {
      if let Some(err) = query.get("error") {
        return Ok(warp::reply::with_status(render_error(format!("An error occurred: {}", err)), warp::http::StatusCode::BAD_REQUEST));
      }
      match query.get("state") {
        Some(check) => if !check.eq(&state) {
          return Ok(warp::reply::with_status(render_error("State mismatch"), warp::http::StatusCode::BAD_REQUEST));
        },
        None =>  return Ok(warp::reply::with_status(render_error("No state provided"), warp::http::StatusCode::BAD_REQUEST)),
      }

      let token_url = match conf.token_url {
        Some(url) => url,
        None      => return Ok(warp::reply::with_status(render_error("No token defined in configuration"), warp::http::StatusCode::INTERNAL_SERVER_ERROR)),
      };
      let code = match query.get("code") {
        Some(code) => code,
        None      => return Ok(warp::reply::with_status(render_error("No authorization code provided"), warp::http::StatusCode::BAD_REQUEST)),
      };
      let url = match Url::parse(&token_url) {
        Ok(url)  => url,
        Err(err) => return Ok(warp::reply::with_status(render_error(format!("Could not parse token URL: {}", err)), warp::http::StatusCode::INTERNAL_SERVER_ERROR)),
      };

      eprintln!("\nRequesting token from:\n    ➤ {}\n", &url);
      let grant_type = match &conf.grant_type {
        Some(grant_type) => grant_type.clone(),
        None             => "code".to_owned(),
      };
      let data = match serde_urlencoded::to_string(&[
        ("grant_type", &grant_type),
        ("code", &code),
        ("client_id", &client.0),
        ("client_secret", &client.1),
      ]) {
        Ok(enc)  => enc,
        Err(err) => return Ok(warp::reply::with_status(render_error(format!("Could not encode token exchange params: {}", err)), warp::http::StatusCode::INTERNAL_SERVER_ERROR)),
      };
      let client = reqwest::Client::new();
      let rsp = match client.post(url.as_str()).header("Content-Type", "application/x-www-form-urlencoded").body(data).send().await {
        Ok(rsp)  => rsp,
        Err(err) => return Ok(warp::reply::with_status(render_error(format!("Could not render template: {}", err)), warp::http::StatusCode::INTERNAL_SERVER_ERROR)),
      };
      let status = rsp.status();
      let data: serde_json::Value = match rsp.json().await {
        Ok(rsp)  => rsp,
        Err(err) => return Ok(warp::reply::with_status(render_error(format!("Could not receive response: {}", err)), warp::http::StatusCode::INTERNAL_SERVER_ERROR)),
      };

      println!("{}", json!(data));
      let rsp = match status {
        reqwest::StatusCode::OK          => warp::reply::with_status(render_response(&data), warp::http::StatusCode::OK),
        reqwest::StatusCode::BAD_REQUEST => warp::reply::with_status(render_error_detail(&data), warp::http::StatusCode::UNAUTHORIZED),
        code                             => warp::reply::with_status(render_error(format!("Request failed with status: {}", code)), warp::http::StatusCode::UNAUTHORIZED),
      };
      match tx.send(()).await {
        Ok(_)  => Ok(rsp),
        Err(_) => Err(warp::reject::reject()),
      }
    });

  let (_, server) = warp::serve(routes)
    .bind_with_graceful_shutdown(([127, 0, 0, 1], 4000), async move {
      rx.recv().await;
    });

  if !opts.passive {
    eprintln!("\nOpening the OAuth2 flow in your browser. If your browser doesn't open, visit the link manually:\n    ➤ {}\n", url.as_str());
    if opts.interactive {
      confirm("\nInitiate the OAuth2 flow? [y/N] ", "y")?;
    }
    open::that(url.as_str())?;
  }

  eprintln!("Waiting for a response from the service...");
  let _ = tokio::task::spawn(server).await;
  Ok(0)
}

fn confirm(prompt: &str, expect: &str) -> Result<(), error::Error> {
  eprint!("{}", prompt);
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

fn render_error<E: Serialize>(data: E) -> warp::reply::Html<String> {
  render_template(TEMPLATE_ERROR, &json!(data))
}

fn render_error_detail(data: &serde_json::Value) -> warp::reply::Html<String> {
  render_template(TEMPLATE_ERROR_DETAIL, data)
}

fn render_response(data: &serde_json::Value) -> warp::reply::Html<String> {
  render_template(TEMPLATE_SUCCESS, data)
}

fn render_template<E: Serialize>(tmpl: &str, data: E) -> warp::reply::Html<String> {
  let reg = handlebars::Handlebars::new();
  match reg.render_template(tmpl, &json!(data)) {
    Ok(rsp)  => warp::reply::html(rsp),
    Err(err) => warp::reply::html(format!("<p>Could not render template: {}</p>", err)),
  }
}

const TEMPLATE_ERROR: &str = r#"<html>
<head>
  <style>
    td {
      font-size: 1.2em;
    }
    td.key {
      font-weight: 700;
      text-align: right;
    }
  </style>
</head>
<body>
  <h1>An error occurred</h1>
  <p>{{ this }}</p>
</body>
</html>
"#;

const TEMPLATE_ERROR_DETAIL: &str = r#"<html>
<head>
  <style>
    td {
      font-size: 1.2em;
    }
    td.key {
      font-weight: 700;
      text-align: right;
    }
  </style>
</head>
<body>
  <h1>An error occurred</h1>
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

const TEMPLATE_SUCCESS: &str = r#"<html>
<head>
  <style>
    td {
      font-size: 1.2em;
    }
    td.key {
      font-weight: 700;
      text-align: right;
    }
  </style>
</head>
<body>
  <h1>Connection created</h1>
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
