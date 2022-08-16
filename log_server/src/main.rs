//! Simple HTTPS echo service based on hyper-rustls
//!
//! First parameter is the mandatory port to use.
//! Certificate and private key are hardcoded to sample files.
//! hyper will automatically use HTTP/2 if a client starts talking HTTP/2,
//! otherwise HTTP/1.1 will be used.
use core::task::{Context, Poll};
use futures_util::{ready, StreamExt};
use hyper::server::accept::Accept;
use hyper::server::conn::{AddrIncoming, AddrStream};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::vec::Vec;
use std::{fs, io, io::Write, sync};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_rustls::rustls::ServerConfig;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use mongodb::{bson::doc, options::ClientOptions, Client};

use tls_server::processor::*;
use tls_server::connection::*;
use tls_server::globals::*;

#[macro_use]
extern crate lazy_static;

// sets up global configuration settings
lazy_static! {
  static ref CONFIG: std::collections::HashMap<String, String> = Globals::new().globals;
}

fn main() {

  // Serve an echo service over HTTPS, with proper error handling.
  if let Err(e) = run_server() {
    eprintln!("FAILED: {}", e);
    std::process::exit(1);
  }
}

fn error(err: String) -> io::Error {
  io::Error::new(io::ErrorKind::Other, err)
}



#[tokio::main]
async fn run_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

  // First parameter is port number (optional, defaults to 1337)
  let addr = format!("0.0.0.0:{}", CONFIG.get("server_port").unwrap()).parse()?;

  // Build TLS configuration.
  let tls_cfg = {
    // Load public certificate.
    let certs = load_certs(CONFIG.get("cert_path").unwrap())?;
    // Load private key.
    let key = load_private_key(CONFIG.get("key_path").unwrap())?;
    // Do not use client certificate authentication.
    let mut cfg = rustls::ServerConfig::builder()
      .with_safe_defaults()
      .with_no_client_auth()
      .with_single_cert(certs, key)
      .map_err(|e| error(format!("{}", e)))?;
    // Configure ALPN to accept HTTP/2, HTTP/1.1 in that order.
    cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    sync::Arc::new(cfg)
  };

  // Create a TCP listener via tokio.
  let incoming = AddrIncoming::bind(&addr)?;
  let service = make_service_fn(|_| async { Ok::<_, io::Error>(service_fn(echo)) });
  let server = Server::builder(TlsAcceptor::new(tls_cfg, incoming)).serve(service);

  // Run the future, keep going until an error occurs.
  println!("Starting Log_server on https://{}", addr);
  server.await?;
  Ok(())
}

enum State {
  Handshaking(tokio_rustls::Accept<AddrStream>),
  Streaming(tokio_rustls::server::TlsStream<AddrStream>),
}

// tokio_rustls::server::TlsStream doesn't expose constructor methods,
// so we have to TlsAcceptor::accept and handshake to have access to it
// TlsStream implements AsyncRead/AsyncWrite handshaking tokio_rustls::Accept first
pub struct TlsStream {
  state: State,
}

impl TlsStream {
  fn new(stream: AddrStream, config: Arc<ServerConfig>) -> TlsStream {
    let accept = tokio_rustls::TlsAcceptor::from(config).accept(stream);
    TlsStream {
      state: State::Handshaking(accept),
    }
  }
}

impl AsyncRead for TlsStream {
  fn poll_read(
    self: Pin<&mut Self>,
    cx: &mut Context,
    buf: &mut ReadBuf,
  ) -> Poll<io::Result<()>> {
    let pin = self.get_mut();
    match pin.state {
      State::Handshaking(ref mut accept) => match ready!(Pin::new(accept).poll(cx)) {
        Ok(mut stream) => {
          let result = Pin::new(&mut stream).poll_read(cx, buf);
          pin.state = State::Streaming(stream);
          result
        }
        Err(err) => Poll::Ready(Err(err)),
      },
      State::Streaming(ref mut stream) => Pin::new(stream).poll_read(cx, buf),
    }
  }
}

impl AsyncWrite for TlsStream {
  fn poll_write(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &[u8],
  ) -> Poll<io::Result<usize>> {
    let pin = self.get_mut();
    match pin.state {
      State::Handshaking(ref mut accept) => match ready!(Pin::new(accept).poll(cx)) {
        Ok(mut stream) => {
          let result = Pin::new(&mut stream).poll_write(cx, buf);
          pin.state = State::Streaming(stream);
          result
        }
        Err(err) => Poll::Ready(Err(err)),
      },
      State::Streaming(ref mut stream) => Pin::new(stream).poll_write(cx, buf),
    }
  }

  fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
    match self.state {
      State::Handshaking(_) => Poll::Ready(Ok(())),
      State::Streaming(ref mut stream) => Pin::new(stream).poll_flush(cx),
    }
  }

  fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
    match self.state {
      State::Handshaking(_) => Poll::Ready(Ok(())),
      State::Streaming(ref mut stream) => Pin::new(stream).poll_shutdown(cx),
    }
  }
}

pub struct TlsAcceptor {
  config: Arc<ServerConfig>,
  incoming: AddrIncoming,
}

impl TlsAcceptor {
  pub fn new(config: Arc<ServerConfig>, incoming: AddrIncoming) -> TlsAcceptor {
    TlsAcceptor { config, incoming }
  }
}

impl Accept for TlsAcceptor {
  type Conn = TlsStream;
  type Error = io::Error;

  fn poll_accept(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
  ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
    let pin = self.get_mut();
    match ready!(Pin::new(&mut pin.incoming).poll_accept(cx)) {
      Some(Ok(sock)) => Poll::Ready(Some(Ok(TlsStream::new(sock, pin.config.clone())))),
      Some(Err(e)) => Poll::Ready(Some(Err(e))),
      None => Poll::Ready(None),
    }
  }
}

fn set_response_error(mut response: Response<Body>, err: String) -> std::result::Result<Response<Body>, hyper::Error> {

  *response.status_mut() = StatusCode::UNAUTHORIZED;
  println!("Error! {}", err);
  *response.body_mut() = Body::from(err);
  Ok(response)

}

async fn get_db_conn(username: &str, password: &str, database: &str) -> std::result::Result<Client, mongodb::error::Error> {
  
  // Connecting to database
  let mut db_conn_string = String::from("mongodb://");
  db_conn_string.push_str(username);
  db_conn_string.push_str(":");
  db_conn_string.push_str(password);
  db_conn_string.push_str("@localhost:27017/?authMechanism=SCRAM-SHA-256");
  db_conn_string.push_str("&authSource=");
  db_conn_string.push_str(database);

  let client_options = ClientOptions::parse(db_conn_string).await?;

  let client = Client::with_options(client_options)?;

  // Perform quick check to see if authenticated
  client.database(database).run_command(doc! {"ping": 1}, None).await?;

  Ok(client)

}

// Custom echo service, handling two different routes and a
// catch-all 404 responder.
async fn echo(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
  
  let mut response = Response::new(Body::empty());

  // getting connection info from request headers
  let conn = match Connection::get_conn_info(&req) {
    Ok(v) => v,
    Err(err) => {
      return set_response_error(response, err.to_string())
    }

  };

  match (req.method(), req.uri().path()) {

    // for the usual uploading of simulation data
    (&Method::POST, "/upload") => {

      // Connecting to database 
      let client = match get_db_conn(&conn.username, &conn.password, CONFIG.get("database").unwrap()).await {
        Ok(c) => c,
        Err(err) => {
          return set_response_error(response, err.to_string())
        }
      };

      // checking if file already exists in database
      let v: Vec<_> = Connection::simple_db_query(&client, "upload_hash", &conn.filehash, CONFIG.get("database").unwrap(), CONFIG.get("registry").unwrap(), None).await.collect().await;
      if v.len() > 0 {
        return set_response_error(response, "File already exists cancelling upload".to_string());
      }

      // Await the full body to be concatenated into a single `Bytes`...
      let full_body = hyper::body::to_bytes(req.into_body()).await?;

      
      // Starting thread here to return response immediately to user
      tokio::spawn(async move {
        let mut new_filename = String::new();
        new_filename.push_str(CONFIG.get("data_path").unwrap());
        new_filename.push_str(&conn.filehash);
        new_filename.push('_');
        new_filename.push_str(&conn.filename);

        let mut outputfile = fs::File::create(&new_filename).expect("File creation failed");
        outputfile.write_all(&full_body).expect("File write failed");
        outputfile.flush().unwrap();
        
        // Leaving server code to process data into database
        let processor = Processor::new(new_filename, conn, client);
        processor.process_data().await.unwrap();

      });
      

      *response.body_mut() = Body::from("Data received");
    }

    // method for checking if record id exists in database
    (&Method::POST, "/check") => {

      // Connecting to database 
      let client = match get_db_conn(&conn.username, &conn.password, CONFIG.get("database").unwrap()).await {
        Ok(c) => c,
        Err(err) => {
          return set_response_error(response, err.to_string())
        }
      };

      // checking if record id already exists in database
      let v: Vec<_> = Connection::simple_db_query(&client, "id", &conn.filehash, CONFIG.get("database").unwrap(), CONFIG.get("registry").unwrap(), None).await.collect().await;
      if v.len() > 0 {
        response.headers_mut().insert("id_exists", hyper::header::HeaderValue::from_str("1").unwrap());
      } else {
        response.headers_mut().insert("id_exists", hyper::header::HeaderValue::from_str("0").unwrap());
      }

    }

    // method for cleaning up files left on disk but deleted from database
    (&Method::POST, "/cleanup") => {

      // Connecting to database      
      let client = match get_db_conn("admin", &conn.password, "admin").await {
        Ok(c) => c,
        Err(err) => {
          return set_response_error(response, err.to_string())
        }
      };

      let files = fs::read_dir(CONFIG.get("data_path").unwrap()).unwrap();
      let mut result_string = String::new();
      
      for f in files {
        let filepath = f.as_ref().unwrap().path().into_os_string();
        let v: Vec<_> = Connection::simple_db_query(&client, "upload_path", filepath.to_str().unwrap(), CONFIG.get("database").unwrap(), CONFIG.get("registry").unwrap(), None).await.collect().await;
        if v.len() == 0 {
          result_string.push_str(filepath.to_str().unwrap());
          result_string.push('\n');
          fs::remove_file(f.unwrap().path()).unwrap();
        }
      }

      *response.body_mut() = Body::from(result_string);
    }

    // method for creating a new user
    (&Method::POST, "/register") => {

      // Connecting to database      
      let client = match get_db_conn("admin", &conn.password, "admin").await {
        Ok(c) => c,
        Err(err) => {
          return set_response_error(response, err.to_string())
        }
      };

      // Creating new password for user
      let new_key: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();
      
      // Adding password to response here, so it will be returned even if cannot make new user. But the password won't be valid
      // if the user is not added
      response.headers_mut().insert("key", hyper::header::HeaderValue::from_str(&new_key).unwrap());

      let user_creation_result = client.database(CONFIG.get("database").unwrap())
      .run_command(doc! {
        "createUser": conn.username,
        "pwd": new_key,
        "roles": [{"role": "readWrite", "db": CONFIG.get("database").unwrap()}]
      }, None)
      .await;

      match user_creation_result {
        Ok(_) => *response.body_mut() = Body::from("New user created successfully"),
        Err(err) => {
          return set_response_error(response, err.to_string())
        }
      };

    }

    // Catch-all 404.
    _ => {
      *response.status_mut() = StatusCode::NOT_FOUND;
    }
  };
  Ok(response)
}

// Load public certificate from file.
fn load_certs(filename: &str) -> io::Result<Vec<rustls::Certificate>> {
  // Open certificate file.
  let certfile = fs::File::open(filename)
    .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
  let mut reader = io::BufReader::new(certfile);

  // Load and return certificate.
  let certs = rustls_pemfile::certs(&mut reader)
    .map_err(|_| error("failed to load certificate".into()))?;
  Ok(certs
    .into_iter()
    .map(rustls::Certificate)
    .collect())
}

// Load private key from file.
fn load_private_key(filename: &str) -> io::Result<rustls::PrivateKey> {
  // Open keyfile.
  let keyfile = fs::File::open(filename)
    .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
  let mut reader = io::BufReader::new(keyfile);

  // Load and return a single private key.
  let keys = rustls_pemfile::rsa_private_keys(&mut reader)
    .map_err(|_| error("failed to load private key".into()))?;
  if keys.len() != 1 {
    return Err(error("expected a single private key".into()));
  }

  Ok(rustls::PrivateKey(keys[0].clone()))
}
