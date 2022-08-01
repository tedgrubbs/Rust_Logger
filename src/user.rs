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

use async_std::task;

const KEY_FILE: &str = "/etc/.Rust_Logger_Credentials";

const CONN_OPTIONS: [&str; 2] = ["Username:", "Server:"];

pub struct User {
  user_id: u32,
  logger_config_path: PathBuf,
  pub db_table: HashMap<String, String>,
  admin_password: String,
  key: String
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

  pub async fn send_data(&self, endpoint: &str, body: Vec<u8>) -> std::result::Result<hyper::HeaderMap<hyper::header::HeaderValue>, hyper::Error> {
    let mut server: String = self.db_table.get("Server:").unwrap().to_string();
    server.insert_str(0, "https://");
    server.push_str(endpoint);

    let req = Request::builder()
    .method(Method::POST)
    .uri(server)
    .header("password", &self.admin_password)
    .header("username", self.db_table.get("Username:").unwrap().to_string())
    .body(Body::from(body)).unwrap();

    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    let resp = client.request(req).await?;
    let status = resp.status();
    println!("{}", status);
    if status != StatusCode::OK {
      let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;
      panic!("Error registering with server: {:?}", body_bytes);
    }

    Ok(resp.headers().to_owned())

  }

  async fn register(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {

    let headers = self.send_data("/register", Vec::new()).await?;
    let new_key = headers.get("key").unwrap().as_bytes();
    println!("Registration with server successful");

    // create new file, overwriting the old. Set permissions
    self.get_root();
    let mut file = fs::File::create(KEY_FILE).expect("error creating new credential file");
    file.set_permissions(fs::Permissions::from_mode(0o600)).expect("Permission set failure");
    file.write_all(new_key).unwrap();
    file.flush().unwrap();
    self.return_root();

    Ok(())
  }

  pub async fn check_creds(&mut self) -> io::Result<()> {

    if !path::Path::new(KEY_FILE).exists() {

      println!("No credential file found. Starting registration process. Please enter the administrator password: ");
      self.admin_password.push_str(&rpassword::read_password().unwrap());
      task::block_on(self.register()).unwrap();

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
      for s in CONN_OPTIONS {
        println!("{}", s);
      }
      println!("");
      panic!();
    }

    let creds = fs::read_to_string(&self.logger_config_path).expect("error reading credential file");

    // parsing credential string to insert values into db_table
    for cred_parameter in CONN_OPTIONS {

      let index = match creds.find(cred_parameter) {
        Some(v) => v,
        None => panic!("Missing parameter in credentials file")
      };

      self.db_table.insert(
        cred_parameter.to_string(),
        creds.split_at(index+cred_parameter.len()).1.split_once('\n').unwrap().0.to_string()
      );
    }


    // for (k,v) in &self.db_table {
    //   println!("{}{}", k,v);
    // }

  }


}
