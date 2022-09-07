//! Simple HTTPS echo service based on hyper-rustls
//!
//! First parameter is the mandatory port to use.
//! Certificate and private key are hardcoded to sample files.
//! hyper will automatically use HTTP/2 if a client starts talking HTTP/2,
//! otherwise HTTP/1.1 will be used.
use core::task::{Context, Poll};
use std::io::Read;
use std::path::PathBuf;
use bson::Document;
use futures_util::{ready, StreamExt, TryStreamExt};
use hyper::server::accept::Accept;
use hyper::server::conn::{AddrIncoming, AddrStream};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::vec::Vec;
use std::{fs, io, io::Write as iowritetrait, sync};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_rustls::rustls::ServerConfig;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use mongodb::{bson::doc, options::ClientOptions, Client};
use nix::unistd;

use html_builder::*;
use cookie::{Cookie, CookieJar};
use std::fmt::Write;

use tls_server::processor::*;
use tls_server::connection::*;
use tls_server::config::*;



#[macro_use]
extern crate lazy_static;

// sets up global configuration settings
lazy_static! {
  static ref CONFIG: std::collections::HashMap<String, String> = Config::new().config;
}

// cookie jar for managing logins
static JAR: global::Global<CookieJar> = global::Global::new();

const HTML_PAGES: [&str; 4] = ["", "query", "collections", "login"];

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

async fn remove_expired_cookies() -> Result<(), Box<dyn std::error::Error>> {

  loop {

    tokio::time::sleep(tokio::time::Duration::from_secs(600)).await;

    println!("Checking for dead cookies");
    let mut dead_cookies: Vec<String> = Vec::new();
    let mut jar = JAR.lock_mut()?;

    for c in jar.iter() {
      if c.expires_datetime().unwrap() < cookie::time::OffsetDateTime::now_utc() {
        dead_cookies.push(c.name().to_string());
      }
    }

    for c in dead_cookies {
      println!("Expired {}", c);
      jar.remove(Cookie::named(c));
    }

  }
}

#[tokio::main]
async fn run_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

  // thread to continously check for expired cookies
  tokio::spawn(async {
    remove_expired_cookies().await.unwrap();
  });

  // First parameter is port number (optional, defaults to 1337)
  let addr = format!("0.0.0.0:{}", CONFIG.get("server_port").unwrap()).parse()?;

  // Build TLS configuration.
  let tls_cfg = {
    
    // Load public certificate.
    let certs = load_certs(CONFIG.get("cert_path").unwrap())?;

    // Load private key.
    let key = load_private_key(CONFIG.get("key_path").unwrap())?;

    // switching back to og user id after getting tls certificates
    let raw_uid = unistd::Uid::current().as_raw();
    unistd::seteuid(unistd::Uid::from_raw(raw_uid)).expect("Error setting initial user id");

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

fn set_response_error(err: &str) -> Box<dyn std::error::Error> {
  let error_box: Box<dyn std::error::Error> = String::from(err).into();
  return error_box
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

async fn echo(req: Request<Body>) -> Result<Response<Body>,  hyper::Error> {
  
  let mut response = Response::new(Body::empty());

  // getting connection info from request headers
  let mut conn = Connection::get_conn_info(&req);
  
  let mut error = String::new();

  // current endpoints use only the element after the first '/'
  // so "/query/crack/" only cares about the "/query" part of the endpoint
  let uri_path = req.uri().path().split("/").nth(1).unwrap().to_owned();

  // normal API for everything else
  if let Result::Err(err) = match (req.method(), uri_path.as_str()) {

    // home page in browser
    (&Method::GET, "") => {
      home_page(&mut response).await 
    },

    // displays files within a collection
    // Also used to download these files through browser
    (&Method::GET, "query") => {
      query(&mut response, &mut conn, req).await 
    },

    (&Method::GET, "download") => {
      download(&mut response, &mut conn, req).await 
    },

    // every other GET will assume is a file in the root web directory
    (&Method::GET, _) => {
      return_file(&mut response, req).await
    },

    (&Method::POST, "login") => {
      web_login(&mut response, &mut conn, req).await
    },

    // for the usual uploading of data
    (&Method::POST, "upload") => {
      upload(&mut response, &mut conn, req).await
    },

    // method for checking if record id exists in database
    (&Method::POST, "check") => {
      check(&mut response, &mut conn).await
    },

    (&Method::POST, "update") => {
      get_latest(&mut response, &mut conn).await
    },

    // method for cleaning up files left on disk but deleted from database
    (&Method::POST, "cleanup") => {
      cleanup(&mut response, &mut conn).await
    },

    // method for creating a new user
    (&Method::POST, "register") => {
      register(&mut response, &mut conn).await
    },

    // Catch-all 404.
    _ => {
      Err(set_response_error("Bruh, there's no page here."))
    },

  } {
    error = err.to_string();
  }
  // }

  if !error.is_empty() {
    println!("Server Error: {}", error);
    *response.status_mut() = StatusCode::UNAUTHORIZED;

    // only return html pages for the normal web pages.
    if HTML_PAGES.contains(&uri_path.as_str()) {
      let buf = build_html(Webpage::ErrorPage, None, Some(&error)).unwrap();
      *response.body_mut() = Body::from(buf.finish());
    } else {
      *response.body_mut() = Body::from(error);
    }
    
  }

  Ok(response)
}

#[derive(PartialEq)]
enum Webpage {
  Home,
  Query,
  ErrorPage,
  LoginRedirect,
}

async fn return_file(response: &mut hyper::Response<Body>, req: Request<Body>) -> Result<(), Box<dyn std::error::Error>> {

  check_cookie(req.headers())?;
  
  let uri_path = req.uri().path().split_once("/").unwrap().1;
  let mut file = fs::File::open(uri_path)?;
  let mut data: Vec<u8> = Vec::new();
  file.read_to_end(&mut data)?;
  *response.body_mut() = Body::from(data);

  Ok(())

}

fn build_html(page: Webpage, list: Option<Vec<String>>, pagename: Option<&str>) -> Result<Buffer, Box<dyn std::error::Error>> {

  let mut buf = Buffer::new();
  writeln!(buf, "<!-- My website -->").unwrap();
  buf.doctype();
  let mut html = buf.html().attr("lang='en'");
  let mut head = html.head();
  writeln!(head.title(), "LAMMPS SERVER").unwrap(); 

  if page == Webpage::LoginRedirect {
    head.meta().attr("http-equiv='refresh' content='0; URL=/query'");
    return Ok(buf)
  }

  let mut body = html.body().attr("style='background-color:#808080;'");

  match pagename {
    Some(name) => writeln!(body.h1(), "{}", name).unwrap(),
    None => writeln!(body.h1(), "Rust_Logger").unwrap()
  }; 

  if page == Webpage::Home {
    let mut form = body.form().attr("action='/login' method='post'");
    form.input().attr("type='password' id='password' name='password'");
    form.input().attr("type='submit' value='Enter'");
  }

  if page == Webpage::Query {

    let mut htmllist = body.ul();

    for collection_name in list.unwrap() {
      let mut file_string = String::from(pagename.unwrap());
      file_string.push_str("/");
      file_string.push_str(&collection_name);

      writeln!(
        htmllist.li().a().attr(
            &format!("href='{}'", file_string)
        ),
        "{}", collection_name,
      ).unwrap()
    }
    
  } 
    
  

  Ok(buf)
}

async fn home_page(response: &mut hyper::Response<Body>) -> Result<(), Box<dyn std::error::Error>> {

  let buf = build_html(Webpage::Home, None, None).unwrap();
  *response.body_mut() = Body::from(buf.finish());
  Ok(())
}

async fn web_login (response: &mut hyper::Response<Body>, conn: &mut Connection, req: Request<Body>) -> Result<(), Box<dyn std::error::Error>> {

  let request_body: Vec<u8> = hyper::body::to_bytes(req.into_body()).await.unwrap().into_iter().collect();
  let form_password = String::from_utf8(request_body).unwrap();
  let form_password = form_password.split_once("=").unwrap().1;

  // Connecting to database to verify user 
  get_db_conn(&conn.username, form_password, "admin").await?;
  
  // creating new cookie
  let headers = response.headers_mut();

  // Creating new random cookie string
  let new_cookie_name: String = thread_rng()
  .sample_iter(&Alphanumeric)
  .take(8)
  .map(char::from)
  .collect();

  // password was correct so can store password in cookie jar
  {
    let mut jar = JAR.lock_mut().unwrap();
    let mut new_cookie = Cookie::new(new_cookie_name.to_owned(), form_password.to_owned());
    let mut now = cookie::time::OffsetDateTime::now_utc();
    now += cookie::time::Duration::seconds(600); // cookie is good for 10 minutes
    new_cookie.set_expires(now);
    jar.add(new_cookie);
  };

  headers.insert(hyper::header::SET_COOKIE, hyper::header::HeaderValue::from_str(&new_cookie_name).unwrap());

  let buf = build_html(Webpage::LoginRedirect, None, None).unwrap();
  
  *response.body_mut() = Body::from(buf.finish());

  Ok(())
}

fn check_cookie(headers: &hyper::HeaderMap) -> Result<String, Box<dyn std::error::Error>> {
  
  let mut password = String::new();
  let jar = JAR.lock_mut().unwrap();
  let mut cookie_err = true;

  // iterates over cookies from http request, to check if it exists in our cookie jar
  for c in headers.get_all("cookie").iter() {

    if let Some(cookie) = jar.get(c.to_str().unwrap()) {

      password = cookie.value().to_string();
      let expiration_time = cookie.expires_datetime();
      if !(expiration_time.is_some() && cookie::time::OffsetDateTime::now_utc() > expiration_time.unwrap()) {
        cookie_err = false;
      }
      break;
    }
  }

  if cookie_err {
    return Err(set_response_error("Either your login has expired or you just aren't authorized."))
  }

  Ok(password)
}

async fn download(response: &mut hyper::Response<Body>, conn: &mut Connection, req: Request<Body>) -> Result<(), Box<dyn std::error::Error>> {
  
  let password = check_cookie(req.headers())?;
  let client = get_db_conn(&conn.username, &password, "admin").await?;

  let uri_path: Vec<&str> = req.uri().path().split("/").collect();


  let download_id = uri_path[2];
  let coll = download_id.split_once(":").unwrap().0;

  let mut cursor = Connection::simple_db_query(&client, Some("id"), Some(download_id), CONFIG.get("database").unwrap(), coll, Some(doc! {"upload_path": 1, "upload_name": 1}), None).await;
  let res = cursor.next().await.unwrap().unwrap();

  let mut file = fs::File::open(res.get_str("upload_path").unwrap())?;
  let mut data: Vec<u8> = Vec::new();
  file.read_to_end(&mut data)?;

  let headers = response.headers_mut();
  let mut disp_string = String::from("attachment; filename=\"");
  disp_string.push_str(res.get_str("upload_name").unwrap());
  disp_string.push_str(".tar.gz\"");
  headers.insert(hyper::header::CONTENT_DISPOSITION, hyper::header::HeaderValue::from_str(&disp_string).unwrap());

  *response.body_mut() = Body::from(data);

  Ok(())

}

async fn query(response: &mut hyper::Response<Body>, conn: &mut Connection, req: Request<Body>) -> Result<(), Box<dyn std::error::Error>> {
  
  let password = check_cookie(req.headers())?;
  let client = get_db_conn(&conn.username, &password, "admin").await?;

  let uri_path: Vec<&str> = req.uri().path().split("/").collect();

  // handling queries based on the size of the path.
  // lengths of 2 and 3 can be handled by the generic html builder. 
  // Any more than that uses a specific html code

  let res = match uri_path.len() {

    // root query endpoint, show collections
    2 => {
      let db = client.database(CONFIG.get("database").ok_or("DB not found")?);
      Some(db.list_collection_names(None).await.unwrap())
    },

    // Have picked a collection, now show uploads within collection
    3 => {

      let cursor = Connection::simple_db_query(&client, None, None, CONFIG.get("database").unwrap(), uri_path[2], Some(doc! { "upload_name": 1 }), None).await;
      let res: Vec<String> = cursor.map(|x| x.unwrap().get_str("upload_name").unwrap().to_string()).collect().await;
      Some(res)

    },

    // Have picked an upload, will use custom webpage to show details
    _ => {

      let doc_path = &uri_path[4..];
      let mut cursor = Connection::simple_db_query(&client, Some("upload_name"), Some(uri_path[3]), CONFIG.get("database").unwrap(), uri_path[2], Some(doc! {}), None).await;
      let res = cursor.next().await.unwrap().unwrap();

      let mut sub_doc: &Document = &res;
      let mut item: Option<&bson::Bson> = None;

      // breaks if we get something that is not a document
      for (name_idx, name) in doc_path.iter().enumerate() {

        let name = name.replace("%20", " "); // replaces the %20 with an actual space character

        if let Ok(new_doc) = sub_doc.get_document(name) {

          sub_doc = new_doc;

        } else { // found something that isn't a document

          // attempts to treat next element in path as an actual file.
          // If this is instead something with a directory structure like /dir1/dir2/file.txt, this will fail
          // If we just grab everything else and join with a '/' then it will work.
          item = Some(sub_doc.get(doc_path[name_idx..].join("/").replace("%20", " ")).unwrap());        
          break

        }
      }

      // special code to handle non-document structures
      if item.is_some() {

        let headers = response.headers_mut();
        headers.insert(hyper::header::CONTENT_TYPE, hyper::header::HeaderValue::from_str("text/html; charset=UTF-8").unwrap()); // forces things to not trigger a download from the browser

        // want to change markdown into nice html
        // lots of requirements needed to make this work
        // Need pandoc installed locally.
        // Need the mathjax directory in the root web directory
        // Right now using the default html template but this could be changed in the future.
        if uri_path.last().unwrap().contains(".md") {
          
          let mut pd = pandoc::new();
          let filestring = item.unwrap().to_string();
          // these crazy splits remove the random quotation marks at the beginning and end
          pd.set_input(pandoc::InputKind::Pipe(filestring.split_at(1).1.split_at(filestring.len()-2).0.to_string())); 
          
          let pd_ext: Vec<pandoc::MarkdownExtension> = Vec::new();
          pd.set_output_format(pandoc::OutputFormat::Html, pd_ext);
          pd.set_output(pandoc::OutputKind::Pipe);
          pd.add_option(pandoc::PandocOption::MathJax(Some("https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-chtml-full.js".to_string())));
          pd.add_option(pandoc::PandocOption::Template(PathBuf::from("default.html5")));

          let pd_out = match pd.execute().unwrap() {
            pandoc::PandocOutput::ToBuffer(s) => s,
            _ => {
              let err: Box<dyn std::error::Error> = String::from("Pandoc parsing broke").into();
              return Err(err);
            }
          };

          *response.body_mut() = Body::from(pd_out);

        } else {

          *response.body_mut() = Body::from(item.unwrap().to_string());

        }
        
        return Ok(())

      }

      // else we now show the all elements within this document

      let doc_keys: Vec<String> = sub_doc.keys().map(|x| x.to_string()).collect();

      // html time
      let mut buf = Buffer::new();
      writeln!(buf, "<!-- My website -->").unwrap();
      buf.doctype();
      let mut html = buf.html().attr("lang='en'");
      let mut head = html.head();
      writeln!(head.title(), "LAMMPS SERVER").unwrap(); 
      let mut body = html.body().attr("style='background-color:#808080;'");
      writeln!(body.h1(), "{}", uri_path.last().unwrap()).unwrap();
       
      let mut htmllist = body.ul();

      for k in doc_keys {

        let val = sub_doc.get(&k).unwrap();

        // if is a Document then we need to list sub docs as list 
        if val.element_type() == bson::spec::ElementType::EmbeddedDocument {
          let mut url_string = uri_path.last().unwrap().to_string();
          url_string.push_str("/");
          url_string.push_str(&k);
          url_string.push_str("/");

          writeln!(
            htmllist.li().a(),
            "{}: ", k
          ).unwrap();

          let mut sub_list = htmllist.ul();
          let val = val.as_document().unwrap();
          let val_keys: Vec<String> = val.keys().map(|x| x.to_string()).collect();

          for vk in val_keys {

            let mut url_post_string = String::from(&vk);
            if url_post_string.starts_with(".") { continue; } // ignore hidden files
            url_post_string.insert_str(0, &url_string);
            writeln!(
              sub_list.li().a().attr(&format!("href='{}'", url_post_string)),
              "{}", vk
            ).unwrap();

          }

        } else { // showing other elements normally

          writeln!(
            htmllist.li().a(),
            "{}: {}", k, val.to_string() 
          ).unwrap()

        }
      }

      // if has id then we can download this
      if sub_doc.get_str("id").is_ok() {
        let mut download_attr = String::from("action='/download/");
        download_attr.push_str(sub_doc.get_str("id").unwrap());
        download_attr.push_str("' method='get'");

        let mut form = body.form().attr(&download_attr);
        form.input().attr("type='submit' value='Download'");
      }
      

      // this only executes on pages showing the collections and the uploads within a collection
      *response.body_mut() = Body::from(buf.finish());
      return Ok(())


    }

    
  };

  let buf = build_html(Webpage::Query, res, Some(uri_path.last().unwrap())).unwrap();
  *response.body_mut() = Body::from(buf.finish());

  Ok(())
}

async fn upload(response: &mut hyper::Response<Body>, conn: &mut Connection, req: Request<Body>) -> Result<(), Box<dyn std::error::Error>> {
  // Connecting to database 
  let client = get_db_conn(&conn.username, &conn.password, CONFIG.get("database").unwrap()).await?;

  // checking if file already exists in database
  let num_entries = Connection::simple_db_query(&client, Some("id"), Some(&conn.filehash), CONFIG.get("database").unwrap(), &conn.collection, None, None).await.count().await;
  if num_entries > 0 {
    return Err(set_response_error("File already exists cancelling upload"))
  }

  // Await the full body to be concatenated into a single `Bytes`...
  let full_body = hyper::body::to_bytes(req.into_body()).await.unwrap();

  
  // Starting thread here to return response immediately to user
  // tokio::spawn(async move {
  let mut new_file_path = String::new();
  new_file_path.push_str(CONFIG.get("data_path").unwrap());
  if &conn.filename != "" {
    new_file_path.push_str(&conn.filename);
  } else {
    new_file_path.push_str(&conn.filehash);
  }
  new_file_path.push_str(".tar.gz");

  // don't want to overwrite files
  // if same name, append datetime
  let path_check = Connection::simple_db_query(&client, Some("upload_path"), Some(&new_file_path), CONFIG.get("database").unwrap(), &conn.collection, None, None).await.count().await;
  if path_check > 0  {
    conn.filename.push('_');
    let curr_local_time = chrono::offset::Local::now().to_string();
    conn.filename.push_str(&curr_local_time.replace(" ", "_"));

    new_file_path.clear();
    new_file_path.push_str(CONFIG.get("data_path").unwrap());
    new_file_path.push_str(&conn.filename);
    new_file_path.push_str(".tar.gz");
  }

  let mut outputfile = fs::File::create(&new_file_path).expect("File creation failed");
  outputfile.write_all(&full_body).expect("File write failed");
  outputfile.flush()?;
  
  // Leaving server code to process data into database
  let processor = Processor::new(new_file_path, conn.clone(), client);
  processor.process_data().await.unwrap();

  // });
  
  let mut body_response = String::from("New file created: ");
  body_response.push_str(&conn.filename);
  *response.body_mut() = Body::from(body_response);

  Ok(())
}

async fn check(response: &mut hyper::Response<Body>, conn: &mut Connection) -> Result<(), Box<dyn std::error::Error>> {
  // Connecting to database 
  let client = get_db_conn(&conn.username, &conn.password, CONFIG.get("database").unwrap()).await?;

  // checking if record id already exists in database
  let coll = conn.filehash.split(':').next().unwrap(); // get collection name from id

  let mut cursor = Connection::simple_db_query(&client, Some("id"), Some(&conn.filehash), CONFIG.get("database").unwrap(), coll, Some(doc! {"upload_name": 1}), None).await;  
  match cursor.try_next().await.unwrap() {
    Some(result) => response.headers_mut().insert("upload_name", hyper::header::HeaderValue::from_str(result.get_str("upload_name").unwrap()).unwrap()),
    None => response.headers_mut().insert("upload_name", hyper::header::HeaderValue::from_str("DNE").unwrap())
  };
  

  Ok(())
}

async fn get_latest(response: &mut hyper::Response<Body>, conn: &mut Connection) -> Result<(), Box<dyn std::error::Error>> {

  // Connecting to database 
  let client = get_db_conn(&conn.username, &conn.password, CONFIG.get("database").unwrap()).await?;

  // checking if record id already exists in database
  let coll = conn.collection.split(':').next().unwrap(); // get collection name from id

  let mut cursor = Connection::simple_db_query(&client, None, None, CONFIG.get("database").unwrap(), coll, Some(doc! {"upload_time": 1, "upload_path": 1}), Some(doc! {"upload_time": -1})).await;  
  
  let record = cursor.next().await.unwrap().unwrap();
  let filepath = record.get_str("upload_path").unwrap();
  let mut file = fs::File::open(filepath)?;
  let mut data: Vec<u8> = Vec::new();
  file.read_to_end(&mut data)?;
  *response.body_mut() = Body::from(data);

  Ok(())
}

async fn cleanup(response: &mut hyper::Response<Body>, conn: &mut Connection) -> Result<(), Box<dyn std::error::Error>> {

  // Connecting to database      
  let client = get_db_conn("admin", &conn.password, "admin").await?;

  let files = fs::read_dir(CONFIG.get("data_path").unwrap())?;
  let mut result_string = String::new();
  let database = client.database(CONFIG.get("database").unwrap());

  // for each file need to check every collection to see if it exists
  // if it doesn't then we delete it
  for f in files {

    let filepath = f.as_ref().unwrap().path().into_os_string();
    let mut in_database = false;

    for collection in database.list_collection_names(None).await? {
      let num_entries = Connection::simple_db_query(&client, Some("upload_path"), filepath.to_str(), CONFIG.get("database").unwrap(), &collection, None, None).await.count().await;
      if num_entries != 0 {
        in_database = true;
        break;
      }
    }

    if !in_database  {
      result_string.push_str(filepath.to_str().unwrap());
      result_string.push('\n');
      fs::remove_file(filepath).unwrap();
    }
    
  }
  

  *response.body_mut() = Body::from(result_string);

  Ok(())
}

async fn register(response: &mut hyper::Response<Body>, conn: &mut Connection) -> Result<(), Box<dyn std::error::Error>> {
  // Connecting to database      
  let client =  get_db_conn("admin", &conn.password, "admin").await?;

  // Creating new password for user
  let new_key: String = thread_rng()
    .sample_iter(&Alphanumeric)
    .take(64)
    .map(char::from)
    .collect();
  
  // Adding password to response here, so it will be returned even if cannot make new user. But the password won't be valid
  // if the user is not added
  response.headers_mut().insert("key", hyper::header::HeaderValue::from_str(&new_key).unwrap());

  client.database(CONFIG.get("database").unwrap())
  .run_command(doc! {
    "createUser": &conn.username,
    "pwd": new_key,
    "roles": [{"role": "readWrite", "db": CONFIG.get("database").unwrap()}]
  }, None)
  .await?;

  Ok(())
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
  let keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
    .map_err(|_| error("failed to load private key".into()))?;
  if keys.len() != 1 {
    return Err(error("expected a single private key".into()));
  }

  Ok(rustls::PrivateKey(keys[0].clone()))
}
