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

const KEY_FILE: &str = "/etc/.Rust_Logger_Credentials";

const LOG_OPTIONS: [&str; 2] = ["Username", "Server"];

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
 

  fn send_data(&self, endpoint: &str, file_info: Option<OutputInfo>) -> Option<hyper::HeaderMap<hyper::header::HeaderValue>> {
    let mut server: String = self.db_table.get("Server").unwrap().to_string();
    server.insert_str(0, "https://");
    server.push_str(endpoint);

    let pword = match endpoint {
      Endpoint::REGISTER => &self.admin_password,
      Endpoint::UPLOAD => &self.key,
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
        let req = req.header("filename", f.filename);
        let req = req.header("filehash", f.hash);
        req.body(Body::from(f.data)).unwrap()
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
      println!("{}", status);
      if status != StatusCode::OK {
        let body_bytes = hyper::body::to_bytes(resp.into_body()).await.unwrap();
        println!("Error: {:?}", body_bytes);
        None
      } else {
        Some(resp.headers().to_owned())
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

  pub fn check_creds(&mut self) -> io::Result<()> {

    if !path::Path::new(KEY_FILE).exists() {

      println!("No credential file found. Starting registration process.\nPlease enter the administrator password: ");
      self.admin_password.push_str(&rpassword::read_password().unwrap());
      self.register().unwrap();

    }

    self.get_root();
    let mut file = fs::File::open(KEY_FILE).expect("error opening credential file");
    file.read_to_string(&mut self.key)?;

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

    let log_options = fs::read_to_string(&self.logger_config_path).expect("error reading credential file");

    for l in log_options.lines() {
      let line: Vec<&str> = l.split_whitespace().collect();
      if line.len() == 0 || line[0].chars().nth(0).unwrap() == '#' {
        continue;
      }

      if !LOG_OPTIONS.contains(&line[0]) {
        panic!("Unknown config parameter found: {}", line[0])
      }

      self.db_table.insert(
        line[0].to_string(),
        line[1].to_string()
      );
    }

    // for (k,v) in &self.db_table {
    //   println!("{}{}", k,v);
    // }

  }


}
