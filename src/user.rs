use std::{fs, io};
use std::io::prelude::*;
use std::os::unix::fs::PermissionsExt;

use nix::unistd;
use rpassword;

const LOGGER_CREDENTIALS_FILE: &str = "/etc/.Rust_Logger_Credentials";

const CREDS_OPTIONS: [&str; 4] = ["Username:", "Password:", "Server:", "Key:"];

pub struct User {
  user_id: u32
}

impl User {

  pub fn user() -> User {
    User {user_id: unistd::Uid::current().as_raw()}
  }

  fn get_root(&self) {
    if let Err(e) = unistd::setuid(unistd::Uid::from_raw(0)) {
      println!("Error setting user id: {e:?}");
    }
  }

  fn return_root(&self) {
    if let Err(e) = unistd::setuid(unistd::Uid::from_raw(self.user_id)) {
      println!("Error setting user id: {e:?}");
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

}