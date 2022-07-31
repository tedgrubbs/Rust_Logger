use log::user::*;
use log::command::*;
use std::env;


fn main() {
  println!();

  let mut user = User::user();
  user.check_creds().unwrap();

  let args = env::args();
  if args.len() == 1 {
    return;
  }
  println!("{:?}", args);

  // let output_info = match cmd.execute() {
  //   Ok(v) => v,
  //   Err(e) => panic!("Error executing command {}", e)
  // };

  // match user.send_data(output_info) {
  //   Ok(()) => (),
  //   Err(err) => panic!("Error sending data {}", err)
  // };


}

