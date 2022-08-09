use std::collections::HashMap;
use std::{env, io, path, process, fs};
use std::io::{Write, Read};
use sha2::{Sha256, Digest};
use hex;
use tar::Builder;
use flate2::write::GzEncoder;
use flate2::Compression;


pub struct Command<'a> {
  cmd_string: String, // full command for Lammps
  input_file_path: path::PathBuf, // location of lammps input file or directory
  file_types: Vec<&'a str> // allowed input filetypes 
}

pub struct OutputInfo {
  pub filename: String,
  pub hash: String,
  pub data: Vec<u8>,
}

impl Command<'_> {

  pub fn command(cmd_string: Vec<String>, file_types: Vec<&str>) -> Command {
    println!("Command string received: {:?}", cmd_string);

    // get full input file path
    let mut input_file_path = String::new();
    for i in 1..cmd_string.len() {
      if cmd_string[i-1] == "-in" {
        input_file_path.insert_str(0, &cmd_string[i]);
      }
    }

    if input_file_path.len() == 0 {
      panic!("Improperly formatted command string. No input file found")
    }

    // Get input file directory path and moving to it to ensure logs are stored there
    let input_file_path = path::Path::new(&input_file_path).to_path_buf();

    // Setting our current working directoy to the location of the input lammps file.
    if input_file_path.parent().unwrap() != path::Path::new("") {

      // if input_file_path is a directory do not move into parent
      let change_dir = match input_file_path.is_dir() {
        true => &input_file_path,
        false => input_file_path.parent().unwrap()
      };

      match env::set_current_dir(change_dir) {
        Ok(()) => (),
        Err(error) => match error.kind() {
          io::ErrorKind::NotFound => {
            panic!("Directory not found. Try switching your current directory or providing the full absolute path.");
          },
          other_error => panic!("Bruh... {:?}", other_error)
        }
      };
    }
    println!("Moved to {}", env::current_dir().unwrap().display());

    let mut real_string = String::new();
    for s in cmd_string {
        real_string.push_str(&s);
        real_string.push_str(" ");
    }

    Command {
      cmd_string: real_string,
      input_file_path,
      file_types
    }

  }

  pub fn execute(&self) -> io::Result<OutputInfo> {

    // Executing lammps command by calling lmp directly through shell command
    let mut cmd = process::Command::new("sh");
    cmd.arg("-c");
    cmd.arg(&self.cmd_string);
    cmd.stdout(process::Stdio::inherit()); // Allows process to write to parent stdout

    println!("\nExecuting {:?}\n", cmd);

    println!("Start of command output:\n");

    match cmd.output() {
      Ok(output) => {
        if output.status.success() {
          println!("\nCommand executed successfully. Control returned to log.");
        }
      },
      Err(err) => panic!("Problem running command {:?}", err)
    };

    Ok(self.compress_and_hash().unwrap())
  }

  fn track_files(&self) -> io::Result<()> {

    let mut tracked_file_hashes: HashMap<String, Vec<u8>> = HashMap::new();

    let files = fs::read_dir(&self.input_file_path)?;

    // gets hash of every file that should be tracked 
    for f in files {

      let filename = f.unwrap().file_name().into_string().unwrap();

      for s in &self.file_types {
        if filename.contains(s) {
          
          let mut file = fs::File::open(&filename)?;
          let mut file_data: Vec<u8> = Vec::new();
          file.read_to_end(&mut file_data)?;
          let hash = Sha256::digest(&file_data);
          tracked_file_hashes.insert(filename, hash.to_vec());
          break;

        }
      }
      
    }

    // Then put all hashes into hidden text file along with one "master" hash that sums up the whole directory
    let mut filenames: Vec<&String> = tracked_file_hashes.keys().collect();
    filenames.sort();
    let mut final_hasher = Sha256::new();
    let mut rev_file = fs::File::create(".rev")?;

    for f in filenames {
      rev_file.write_all(f.as_bytes())?;
      rev_file.write_all(b" ")?;
      rev_file.write_all(hex::encode(tracked_file_hashes.get(f).unwrap()).as_bytes())?;
      rev_file.write_all(b"\n")?;
      final_hasher.update(tracked_file_hashes.get(f).unwrap());
    }
    let final_hash = final_hasher.finalize();
    rev_file.write_all(b"id ")?;
    rev_file.write_all(hex::encode(final_hash).as_bytes())?;
    rev_file.flush()?;
    

    Ok(())
  }

  pub fn compress_and_hash(&self) -> io::Result<OutputInfo> {
    
    self.track_files().unwrap();

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
    let hash = Sha256::digest(&compressed_data);
    println!("File hash: {}", hex::encode(hash));

    // let mut compressed_data_file = fs::File::create(&output_filename)?;
    // compressed_data_file.write_all(&compressed_data)?;

    let command_outputs = OutputInfo {
      filename: output_filename,
      hash: hex::encode(hash),
      data: compressed_data,
    };

    Ok(command_outputs)
  }

}
