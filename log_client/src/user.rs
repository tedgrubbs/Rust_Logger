use std::path::PathBuf;
use std::{fs, io, path, collections::HashMap};
use std::io::prelude::*;
use std::os::unix::fs::PermissionsExt;

use nix::unistd;
use home;
use rpassword;

use hyper::Client;
use hyper::{Body, Method, Request,StatusCode};
use hyper_tls::HttpsConnector;

use tokio::runtime::Runtime;
use crate::command::OutputInfo;
use utils::utils;

const KEY_FILE: &str = "/etc/.Rust_Logger_Credentials";

const LOG_OPTIONS: [&str; 3] = ["Username", "Server", "tracked_files"];

pub struct User {
  user_id: u32,
  logger_config_path: PathBuf,
  pub db_table: HashMap<String, String>,
  admin_password: String,
  key: String
}

// Lists possible endpoints on server
struct Endpoint{}
impl<'a> Endpoint {
  const REGISTER: &'a str = "/register";
  const UPLOAD: &'a str = "/upload";
  const CLEANUP: &'a str = "/cleanup";
  const ID_CHECK: &'a str = "/check";
}

impl User {

  // When starting as root suid, effective id is root. Want to turn this off until needed
  pub fn user() -> User {
    let raw_uid = unistd::Uid::current().as_raw();
    unistd::seteuid(unistd::Uid::from_raw(raw_uid)).expect("Error setting initial user id");

    let mut logger_config_path = home::home_dir().unwrap();
    logger_config_path.push(".log/config");

    let mut new_user = User {
      user_id: raw_uid,
      logger_config_path: logger_config_path,
      db_table: HashMap::new(),
      admin_password: String::new(), // admin_password should be blank unless performing registration
      key: String::new() // key will be initialized at check_creds()
    };

    new_user.read_config_file();

    // quick test to see if we can get root
    new_user.get_root();
    new_user.return_root();

    new_user

  }

  fn get_root(&self) {
    if let Err(e) = unistd::seteuid(unistd::Uid::from_raw(0)) {
      println!("Error setting root id: {e:?}. Executable was probably not compiled as an SUID binary");
      panic!();
    }
  }

  fn return_root(&self) {
    if let Err(e) = unistd::seteuid(unistd::Uid::from_raw(self.user_id)) {
      println!("Error setting original user id: {e:?}");
      panic!();
    }
  }

  pub fn send_output(&self, output_info: OutputInfo) {
    self.send_data(Endpoint::UPLOAD, Some(output_info));
  }

  pub fn check_id(&self, output_info: OutputInfo) -> String {
    let result = self.send_data(Endpoint::ID_CHECK, Some(output_info)).unwrap();
    result.get("upload_name").unwrap().to_str().unwrap().to_string()
  }
 

  fn send_data(&self, endpoint: &str, file_info: Option<OutputInfo>) -> Option<hyper::HeaderMap<hyper::header::HeaderValue>> {
    let mut server: String = self.db_table.get("Server").unwrap().to_string();
    server.insert_str(0, "https://");
    server.push_str(endpoint);

    let pword = match endpoint {
      Endpoint::REGISTER => &self.admin_password,
      Endpoint::CLEANUP => &self.admin_password,
      Endpoint::UPLOAD => &self.key,
      Endpoint::ID_CHECK => &self.key,
      _ => ""
    };
    

    let req = Request::builder()
    .method(Method::POST)
    .uri(server)
    .header("password", pword)
    .header("username", self.db_table.get("Username").unwrap().to_string());
    
    let req = match file_info {
      None => req.body(Body::from("")).unwrap(),
      Some(f) => {

        let req = req.header("collection", f.collection_name.unwrap());

        match endpoint {
          // can just reuse the filehash header for this
          Endpoint::ID_CHECK => {
            let req = req.header("filehash", f.record_file_hash.unwrap());
            req.body(Body::from("")).unwrap()
          },
          _ => {
            let req = req.header("filename", f.filename.unwrap());
            let req = req.header("filehash", f.hash.unwrap());
            req.body(Body::from(f.compressed_dir.unwrap())).unwrap()
          }
        }
      }
    };

    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    // No need to make entire program asynchronous so just defining runtime here to keep it isolated.
    // Runtime creation takes only 1 or 2 milliseconds
    let rt = Runtime::new().unwrap();
    let resp = rt.block_on(async move {

      let resp = client.request(req).await.unwrap();
      let status = resp.status();
      

      let headers = resp.headers().to_owned();
      let body_bytes = hyper::body::to_bytes(resp.into_body()).await.unwrap();
      let body_string = std::str::from_utf8(&body_bytes).unwrap();

      if status != StatusCode::OK {
        println!("Error: {}", body_string);
        None
      } else {
        println!("{}", body_string);
        Some(headers)
      }
      
    });
    
    resp
  }

   fn register(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {

    let headers = self.send_data(Endpoint::REGISTER, None).unwrap();
    let new_key = headers.get("key").unwrap().as_bytes();
    println!("Registration with server successful\n");

    // create new file, overwriting the old. Set permissions
    self.get_root();
    let mut file = fs::File::create(KEY_FILE).expect("error creating new credential file");
    file.set_permissions(fs::Permissions::from_mode(0o600)).expect("Permission set failure");
    file.write_all(new_key).unwrap();
    file.flush().unwrap();
    self.return_root();

    Ok(())
  }

  pub fn clean_up(&mut self) {
    println!("\nPlease enter the administrator password: ");
      self.admin_password.push_str(&rpassword::read_password().unwrap());
      self.send_data(Endpoint::CLEANUP, None).unwrap();
  }

  pub fn check_creds(&mut self) -> io::Result<()> {

    if !path::Path::new(KEY_FILE).exists() {

      println!("No credential file found. Starting registration process.\nPlease enter the administrator password: ");
      self.admin_password.push_str(&rpassword::read_password().unwrap());
      self.register().unwrap();

    }

    self.get_root();
    let mut file = fs::File::open(KEY_FILE).expect("error opening credential file");
    file.read_to_string(&mut self.key)?;
    self.return_root();

    println!("Key found on local system");
    Ok(())
  }

  fn read_config_file(&mut self) {

    // checking if credentials file exists
    if !path::Path::new(&self.logger_config_path).exists() {
      println!("Error: credentials not set up. Cannot log data before setup.");
      println!("Please create a file at ~/.log/config with the connection details like so:");
      for s in LOG_OPTIONS {
        println!("{} <value>", s);
      }
      println!("");
      panic!();
    }

    utils::read_file_into_hash(self.logger_config_path.to_str().unwrap(), Some(&LOG_OPTIONS), &mut self.db_table).unwrap();

    // for (k,v) in &self.db_table {
    //   println!("{} {}", k,v);
    // }

  }


}
