use std::{fs, io, path, collections::HashMap};
use std::io::prelude::*;
use std::os::unix::fs::PermissionsExt;

use nix::unistd;
use rpassword;

const LOGGER_CREDENTIALS_FILE: &str = "/etc/.Rust_Logger_Credentials";

const CREDS_OPTIONS: [&str; 4] = ["Username:", "Password:", "Server:", "Key:"];

pub struct User {
  user_id: u32,
  db_table: HashMap<String, String>,
}

impl User {

  // When starting as root suid, effective id is root. Want to turn this off until needed
  pub fn user() -> User {
    let raw_uid = unistd::Uid::current().as_raw();
    unistd::seteuid(unistd::Uid::from_raw(raw_uid)).expect("Error setting initial user id");
    User {
      user_id: raw_uid,
      db_table: HashMap::new()
    }
    
  }

  fn get_root(&self) {
    if let Err(e) = unistd::seteuid(unistd::Uid::from_raw(0)) {
      println!("Error setting root id: {e:?}");
      panic!();
    }
  }

  fn return_root(&self) {
    if let Err(e) = unistd::seteuid(unistd::Uid::from_raw(self.user_id)) {
      println!("Error setting original user id: {e:?}");
      panic!();
    }
  }

  pub fn generate_creds_file(&self) {

    self.get_root();

    // create new file, overwriting the old. Set permissions
    let mut file = fs::File::create(LOGGER_CREDENTIALS_FILE).expect("error creating new credential file");
    file.set_permissions(fs::Permissions::from_mode(0o600)).expect("Permission set failure");

    let mut user_input = String::new();

    for (i, &s) in CREDS_OPTIONS.iter().enumerate() {

      println!("{}", s);
      // if getting password or private key, obfuscate input
      if i == 1 || i == 3 {
        let password = rpassword::read_password().unwrap();
        user_input.insert_str(0, &password);
        user_input.insert_str(user_input.len(), "\n");
      } else {
        io::stdin().read_line(&mut user_input).expect("Failed to read line");
      }

      user_input.insert_str(0, s);
      file.write(user_input.as_bytes()).expect("File write failed");
      user_input.clear();

    }

    self.return_root();

  }

  pub fn read_creds_file(&mut self) {

    // checking if credentials file exists
    if !path::Path::new(LOGGER_CREDENTIALS_FILE).exists() {
      println!("Error: credentials not set up. Cannot log data before setup.");
      return
    }

    self.get_root();
    let creds = fs::read_to_string(LOGGER_CREDENTIALS_FILE).expect("error reading credential file");
    self.return_root();
    
    // parsing credential string to insert values into db_table
    for (_, &cred_parameter) in CREDS_OPTIONS.iter().enumerate() {

      let index = match creds.find(cred_parameter) {
        Some(v) => v,
        None => panic!("Missing parameter in credentials file")
      };

      self.db_table.insert(
        cred_parameter.to_string(),
        creds.split_at(index+cred_parameter.len()).1.split_once('\n').unwrap().0.to_string()
      ) ; 
    }
  }


}