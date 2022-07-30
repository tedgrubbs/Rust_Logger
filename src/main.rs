use log::user::*;
use log::command::*;
use std::env;

#[tokio::main]
async fn main() {
  println!();

  let mut user = User::user();
  user.check_creds();

  // let output_info = match cmd.execute() {
  //   Ok(v) => v,
  //   Err(e) => panic!("Error executing command {}", e)
  // };

  // match user.send_data(output_info) {
  //   Ok(()) => (),
  //   Err(err) => panic!("Error sending data {}", err)
  // };

  


}

