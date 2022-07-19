use std::{env};

mod user;
use user::*;



fn main() {

  let user = User::user();
  
  let args: Vec<String> = env::args().collect();

  if args.len() == 1 || args[1] == "-h" {
    println!("Usage:");
    println!("log <command> // logs command and logs to server");
    println!("log -init // Creates credential file on disk");
    return
  }

  // checking if credentials file exists
  // if !path::Path::new(LOGGER_CREDENTIALS_FILE).exists() {
  //   println!("Error: credentials not set up. Cannot log data before setup.");
  //   return
  // }



  if args[1] == "-init" {
    user.generate_creds_file();
  }

}
