use clap::{Parser};
use log::user::*;
use log::command::*;

#[derive(Parser)]
struct Args {
  /// Run this first or if you want to change database server parameters
  #[clap(short, long, action)]
  init: bool,

//clap(short,long,action, init:bool)

  /// Enter a string here as the command you wish to run and log
  #[clap(short, long, value_parser)]
  command: Option<String>,
}



fn main() {
  println!();

  let mut user = User::user();
 
  let args = Args::parse();


  if args.init {
    user.generate_creds_file();
  }

  user.read_creds_file();
  
  let cmd = match args.command {
    Some(x) => {
      Command::command(x)
    },
    None => {
      println!("Please enter a command");
      return
    },
  };

  let output_info = match cmd.execute() {
    Ok(v) => v,
    Err(e) => panic!("Error executing command {}", e)
  };

  match user.send_data(output_info) {
    Ok(()) => (),
    Err(err) => panic!("Error sending data {}", err)
  };

  


}

