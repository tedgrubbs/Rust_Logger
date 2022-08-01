use log::user::*;
use log::command::*;
use std::env;

#[tokio::main]
async fn main() {
  println!();

  let mut user = User::user();
  user.check_creds().await.unwrap();

  let mut args: Vec<String> = env::args().collect();

  if args.len() == 1 {
    println!("Try entering a command like:");
    println!("log mpirun -np 4 lmp -in in.crack");
    println!("Or to just compress and send a directory use log -c ");
    return;
  }

  args.remove(0);

  // Can just compress directory and send it if simulation has already ran
  let mut compress_only = false;
  if args[0] == "-c" {
    args.remove(0);
    compress_only = true;
  }

  let cmd = Command::command(args);

  let output_info = match compress_only {
    false => cmd.execute().unwrap(),
    true => cmd.compress_and_hash().unwrap()
  };
  


  user.send_output(output_info).await;


}
