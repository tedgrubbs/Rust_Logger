use log::user::*;
use log::command::*;
use std::{env};


fn main() {
  println!();

  let mut user = User::user();
  user.check_creds().unwrap();

  let mut args: Vec<String> = env::args().collect();

  if args.len() == 1 {
    println!("\nTry entering a command like:");
    println!("log mpirun -np 4 lmp -in in.crack\n");
    println!("Or to just compress and send a directory:\nlog -c -in <file or directory>\n");
    println!("To clean up dead files on the server:\nlog clean");
    return;
  }

  args.remove(0);

  if args[0] == "clean" {
    args.remove(0);
    user.clean_up();
    return;
  }

  // Can just compress directory and send it if simulation has already ran
  let mut compress_only = false;
  if args[0] == "-c" {
    args.remove(0);
    compress_only = true;
  }

  let cmd = Command::command(args, user.db_table.get("tracked_files").unwrap().split_whitespace().collect());

  let output_info = match compress_only {
    false => cmd.execute().unwrap(),
    true => cmd.compress_and_hash().unwrap()
  };
  


  user.send_output(output_info);

}
