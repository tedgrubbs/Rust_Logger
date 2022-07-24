use std::{env, io, path, process, fs};
use std::io::{Write};
use sha2::{Sha256, Digest};
use hex;
use tar::Builder;
use flate2::write::GzEncoder;
use flate2::Compression;


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

    // Setting our current working directoy to the location of the input lammps file.
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

    // Executing lammps command by calling lmp directly through shell command
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
    let mut output_filename = String::new();
    output_filename.push_str(self.input_file_path.file_name().unwrap().to_str().unwrap());
    output_filename.push_str(".tar.gz");

    // Creating tar archive of directory, then compressing
    println!("Compressing output data.");
    let mut archive = Builder::new(Vec::new());
    archive.append_dir_all("",".").unwrap();
    let archive_result = archive.into_inner().unwrap();
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&archive_result)?;
    let compressed_data = encoder.finish()?;

    println!("\nCalculating file hash...");
    let mut hasher = Sha256::new();
    hasher.update(&compressed_data);
    let hash = hasher.finalize();
    println!("File hash: {}", hex::encode(hash));

    let mut compressed_data_file = fs::File::create(&output_filename)?;
    compressed_data_file.write_all(&compressed_data)?;

    let command_outputs = OutputInfo {
      filename: output_filename,
      hash: hex::encode(hash)
    };

    Ok(command_outputs)
  }

}
