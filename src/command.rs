use std::{env, io, path, process};
use std::io::{Write};

pub struct Command {
  
  cmd_string: String,
  input_file_path: path::PathBuf,
  
}

impl Command {

  pub fn command(cmd_string: String) -> Command {
    println!("Command string received: {}", cmd_string);

    // Get startup working directory
    let starting_dir = env::current_dir().unwrap();
    println!("Starting directory: {}", starting_dir.display());

    // Split out full command string
    let split_string: Vec<&str> = cmd_string.split_whitespace().collect();

    // get full input file path
    let mut input_file_path = String::new();
    for i in 2..split_string.len() {
      if split_string[i-2] == "lmp" && split_string[i-1] == "<" {
        input_file_path.insert_str(0, split_string[i]);
      }
    }


    // Get input file directory path and moving to it to ensure logs are stored there
    let input_file_path = path::Path::new(&input_file_path).to_path_buf();

    Command {
      cmd_string,
      input_file_path
    }
    
    
  }

  pub fn execute(&self) -> io::Result<()> {

    match env::set_current_dir(self.input_file_path.parent().unwrap()) {
      Ok(()) => (),
      Err(error) => match error.kind() {
        io::ErrorKind::NotFound => {
          panic!("Directory not found. Try switching your current directory or providing the full absolute path."); 
        },
        other_error => panic!("Bruh... {:?}", other_error)
      }
    };
    
    println!("Moved to {}", env::current_dir()?.display());

    // Executing command
    let mut cmd = process::Command::new("sh");
    cmd.arg("-c");
    cmd.arg(&self.cmd_string);

    println!("Executing {:?}\n", cmd);

    println!("Start of command output:\n");

    match cmd.output() {
      Ok(output) => {
        io::stdout().write_all(&output.stdout).unwrap();
        if output.status.success() {
          println!("\nCommand executed successfully. Control returned to log.");
        }
      },
      Err(err) => panic!("Problem running command {:?}", err)
    };

    Ok(())
  }

}