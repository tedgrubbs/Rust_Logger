use std::collections::HashMap;
use std::{env, io, path, process, fs};
use std::io::{Write, Read};
use sha2::{Sha256, Digest};
use hex;
use tar::Builder;
use flate2::write::GzEncoder;
use flate2::Compression;

use utils::utils;

const HASH_TRUNCATE_LENGTH: usize = 16;

pub struct Command<'a> {
  cmd_string: String, // full command for Lammps
  input_file_path: path::PathBuf, // location of lammps input file or directory
  file_types: Vec<&'a str>, // allowed input filetypes 
  curr_file_hashes: HashMap<String,String>,
  record_file_hashes: HashMap<String,String>,
  pub needs_update: bool
}

pub struct OutputInfo {
  pub filename: Option<String>,
  pub hash: Option<String>,
  pub compressed_dir: Option<Vec<u8>>,
  pub record_file_hash: Option<String>,
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
      file_types,
      curr_file_hashes: HashMap::new(),
      record_file_hashes: HashMap::new(),
      needs_update: false
    }

  }

  pub fn execute(&mut self) -> io::Result<OutputInfo> {

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

  // Gets hashes for current files in working directory
  fn get_current_filehashes(&mut self) -> io::Result<()> {

    // reads all file names into vec and sorts so the final hash will be deterministic
    let files = fs::read_dir(env::current_dir()?)?;
    let mut filenames: Vec<String> = Vec::new();
    for f in files {
      filenames.insert(0, f.unwrap().file_name().into_string().unwrap());
    }
    filenames.sort();

    let mut final_hasher = Sha256::new();

    // gets hash of every file that should be tracked 
    for f in filenames {

      for s in &self.file_types {
        if f.contains(s) {
          
          let mut file = fs::File::open(&f)?;
          let mut file_data: Vec<u8> = Vec::new();
          file.read_to_end(&mut file_data)?;
          let hash = Sha256::digest(&file_data);
          final_hasher.update(hash);
          self.curr_file_hashes.insert(f, hex::encode(hash)[..HASH_TRUNCATE_LENGTH].to_string());
          break;

        }
      }
    }
    let final_hash = final_hasher.finalize();
    self.curr_file_hashes.insert("id".to_string(), hex::encode(final_hash)[..HASH_TRUNCATE_LENGTH].to_string());

    Ok(())
  }


  fn get_record_filehashes(&mut self)  {
    self.record_file_hashes.clear();
    utils::read_file_into_hash(".rev", None, &mut self.record_file_hashes).unwrap();
  }

  pub fn update_record(&mut self) {

    // current id becomes the new parent id
    self.update_rev_file(Some(self.record_file_hashes.get("id").unwrap())).unwrap();

    // update record filehashes
    self.get_record_filehashes();
  }

  fn update_rev_file(&self, parent_id: Option<&str>) -> io::Result<()> {
    // Puts all hashes into hidden text file along with one "master" hash that sums up the whole directory
    let mut filenames: Vec<&String> = self.curr_file_hashes.keys().collect();
    filenames.sort();

    let mut rev_file = fs::File::create(".rev")?;

    for f in filenames {
      rev_file.write_all(f.as_bytes())?;
      rev_file.write_all(b" ")?;
      rev_file.write_all(self.curr_file_hashes.get(f).unwrap().as_bytes())?;
      rev_file.write_all(b"\n")?;
    }

    rev_file.write_all(b"parent_id ")?;
    match parent_id {
      Some(s) => rev_file.write_all(s.as_bytes())?,
      None => rev_file.write_all(b"*")?
    };
    rev_file.flush()?;
    Ok(())
  }

  pub fn track_files(&mut self) -> io::Result<OutputInfo> {

    self.get_current_filehashes().unwrap();

    // if .rev file exists, get those recorded hashes, otherwise need to create it
    // will return immediately after creating new rev file
    if path::Path::new(".rev").exists() {
      self.get_record_filehashes();

      // can now check if there are any discrepencies between recorded and current filehashes
      self.check_hashes();
      if self.needs_update {
        println!("\nDiscrepency found, need to update record\n");
      } else {
        println!("\nNo changes detected in tracked files\n");
      }

    } else {
      println!("No .rev file found, creating a new one");
      self.update_rev_file(None).unwrap();
      self.get_record_filehashes();
    }

    let basic_info = OutputInfo {
      filename: None,
      hash: None,
      compressed_dir: None,
      record_file_hash: Some(self.record_file_hashes.get("id").unwrap().to_string())
    };

    Ok(basic_info)

   }

  fn check_hashes(&mut self) {

    for (k,v) in &self.curr_file_hashes {
      
      // if record doesn't exist or is different, need to update record
      let record_hash = match self.record_file_hashes.get(k) {
        Some(record) => record,
        None => "None"
      };

      if record_hash != v {
        self.needs_update = true;
        break;
      }

    }

  }

  pub fn compress_and_hash(&mut self) -> io::Result<OutputInfo> {
    
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
      filename: Some(output_filename),
      hash: Some(hex::encode(hash)),
      compressed_dir: Some(compressed_data),
      record_file_hash: None
    };

    Ok(command_outputs)
  }

}
