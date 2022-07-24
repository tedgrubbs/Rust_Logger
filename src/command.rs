use std::{env, io, path, process, fs};
use std::io::{Write};
use sha2::{Sha256, Digest};
use hex;

pub struct Command {
  cmd_string: String,
  input_file_path: path::PathBuf,
}

pub struct OutputInfo {
  pub filename: String,
  pub hash: String
}

impl Command {

  pub fn command(cmd_string: String) -> Command {
    println!("Command string received: {}", cmd_string);

    // Get startup working directory
    // let starting_dir = env::current_dir().unwrap();
    // println!("Starting directory: {}", starting_dir.display());

    // Split out full command string
    let split_string: Vec<&str> = cmd_string.split_whitespace().collect();

    // get full input file path
    let mut input_file_path = String::new();
    for i in 2..split_string.len() {
      if split_string[i-2] == "lmp" && split_string[i-1] == "<" {
        input_file_path.insert_str(0, split_string[i]);
      }
    }
    
    if input_file_path.len() == 0 {
      panic!("Improperly formatted command string. No input file found")
    }

    // Get input file directory path and moving to it to ensure logs are stored there
    let input_file_path = path::Path::new(&input_file_path).to_path_buf();

    Command {
      cmd_string,
      input_file_path
    }


  }

  pub fn execute(&self) -> io::Result<OutputInfo> {

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

    println!("\nExecuting {:?}\n", cmd);

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

    // Compressing output directory
    // is this stupid? I think it's the easiest way unless i can find something decent in Rust.
    let mut compress_data_cmd_str = String::new();
    let mut output_filename = String::new();
    output_filename.push_str(self.input_file_path.file_name().unwrap().to_str().unwrap());
    output_filename.push_str(".tar.gz");

    compress_data_cmd_str.insert_str(0, "tar -czf ");
    compress_data_cmd_str.push_str(&output_filename);
    compress_data_cmd_str.push_str(" ");
    compress_data_cmd_str.push_str("*");

    let mut cmd = process::Command::new("sh");
    cmd.arg("-c");
    cmd.arg(&compress_data_cmd_str);

    println!("\nExecuting {:?}\n", cmd);

    // A system process can fail without creating a rust error. So you need to check the output status
    match cmd.output() {
      Ok(output) => {
        io::stdout().write_all(&output.stdout).unwrap();
        if output.status.success() {
          println!("Compressed successfully");
        } else {
          panic!("Compression failed");
        }
      },
      Err(err) => panic!("Well that didn't work {:?}", err)
    }

    println!("\nCalculating file hash...");
    let mut file = fs::File::open(&output_filename)?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)?;
    let hash = hasher.finalize();
    println!("File hash: {}", hex::encode(hash));

    let command_outputs = OutputInfo {
      filename: output_filename,
      hash: hex::encode(hash)
    };
    
    Ok(command_outputs)
  }

}
