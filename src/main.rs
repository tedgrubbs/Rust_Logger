use std::{env};

use log::user::*;

fn main() {

  let mut user = User::user();
 
  let args: Vec<String> = env::args().collect();

  if args.len() == 1 || args[1] == "-h" {
    println!("Usage:");
    println!("log <command> // logs command and logs to server");
    println!("log -init // Creates credential file on disk");
    return
  }

  if args[1] == "-init" {
    user.generate_creds_file();
  }

  user.read_creds_file();


}
