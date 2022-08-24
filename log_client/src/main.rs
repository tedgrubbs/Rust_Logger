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
    compress_only = true;
  }


  let mut filename = String::new();
  match args.iter().position(|x| x == "--name") {
    Some(v) => {
      filename = args[v+1].to_string();
      args.remove(v);
      args.remove(v);
    },
    None => ()
  };
  println!("{:?}", args);

  let mut cmd = Command::command(args, user.db_table.get("tracked_files").unwrap().split_whitespace().collect());
  let tracking_info = cmd.track_files().unwrap();

  // if need to update record, should communicate with server to check if current record id exists
  let og_upload_name = user.check_id(tracking_info);
  if og_upload_name != "DNE" {
    if cmd.needs_update {
      println!("Record exists, can update");
      cmd.update_record()
    }
  } else if cmd.record_file_hashes.get("parent_id").unwrap() != "*" { // if parent id is * then it's a new branch and there is no problem
    println!("Error: Previous record not found in database, revert changes or delete REV file to create a new branch");
    return
  }
    
  let mut output_info = match compress_only {
    false => cmd.execute().unwrap(),
    true => cmd.compress_and_hash().unwrap()
  };

  // if record does not exist, use currently provided filename 
  // if does exists and no new filename is given, use old filename
  if og_upload_name != "DNE" && filename.is_empty() {
    output_info.filename = Some(og_upload_name);
  } else {
    output_info.filename = Some(filename);
  }
  
  user.send_output(output_info);
  

}
