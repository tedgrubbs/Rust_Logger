use log::user::*;
use log::command::*;
use std::env;


fn main() {
  println!();

  let mut user = User::user();
  user.check_creds().unwrap();

  let mut args: Vec<String> = env::args().collect();

  if args.len() == 1 {
    println!("Try entering a command like:");
    println!("log mpirun -np 4 lmp -in in.crack");
    return;
  }

  args.remove(0);
  let cmd = Command::command(args);
  let output_info = match cmd.execute() {
    Ok(v) => v,
    Err(e) => panic!("Error executing command {}", e)
  };


  match user.send_data(output_info) {
    Ok(()) => (),
    Err(err) => panic!("Error sending data {}", err)
  };


}
