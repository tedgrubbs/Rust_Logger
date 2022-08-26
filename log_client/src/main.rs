use std::collections::HashMap;
use std::{env, io, path, process, fs};
use std::io::{Write, Read};
use sha2::{Sha256, Digest};
use hex;
use tar::Builder;
use flate2::write::GzEncoder;
use flate2::Compression;

use utils::utils;

use std::path::PathBuf;
use std::os::unix::fs::PermissionsExt;

use nix::unistd;
use home;
use rpassword;

use hyper::Client;
use hyper::{Body, Method, Request,StatusCode};
use hyper_tls::HttpsConnector;

use tokio::runtime::Runtime;

const HASH_TRUNCATE_LENGTH: usize = 16;

const KEY_FILE: &str = "/etc/.Rust_Logger_Credentials";

const LOG_OPTIONS: [&str; 3] = ["Username", "Server", "tracked_files"];


struct User {
  user_id: u32,
  logger_config_path: PathBuf,
  db_table: HashMap<String, String>,
  admin_password: String,
  key: String,

  cmd_string: String, // full command for Lammps
  input_file_path: path::PathBuf, // location of lammps input file or directory
  curr_file_hashes: HashMap<String,String>, // stores hashes of files  currently in directory
  record_file_hashes: HashMap<String,String>, // stores hashes of files found in REV file
  needs_update: bool, // whether or not the REV file needs to be updated
  
  filename: Option<String>, 
  collection_name: String, // bottom directory name
  hash: Option<String>,
  compressed_dir: Option<Vec<u8>>,
  record_file_hash: Option<String>,
}

// Lists possible endpoints on server
struct Endpoint{}
impl<'a> Endpoint {
  const REGISTER: &'a str = "/register";
  const UPLOAD: &'a str = "/upload";
  const CLEANUP: &'a str = "/cleanup";
  const ID_CHECK: &'a str = "/check";
}

impl User {

  // When starting as root suid, effective id is root. Want to turn this off until needed
  pub fn user() -> User {
    let raw_uid = unistd::Uid::current().as_raw();
    unistd::seteuid(unistd::Uid::from_raw(raw_uid)).expect("Error setting initial user id");

    let mut logger_config_path = home::home_dir().unwrap();
    logger_config_path.push(".log/config");

    let mut new_user = User {
      user_id: raw_uid,
      logger_config_path: logger_config_path,
      db_table: HashMap::new(),
      admin_password: String::new(), // admin_password should be blank unless performing registration
      key: String::new(), // key will be initialized at check_creds()

      cmd_string: String::new(), // full command for Lammps
      input_file_path: path::PathBuf::new(), // location of lammps input file or directory
      curr_file_hashes: HashMap::new(), // stores hashes of files  currently in directory
      record_file_hashes: HashMap::new(), // stores hashes of files found in REV file
      needs_update: false, // whether or not the REV file needs to be updated

      filename: None, 
      collection_name: String::new(), // bottom directory name
      hash: None,
      compressed_dir: None,
      record_file_hash: None
    };

    new_user.read_config_file();

    // quick test to see if we can get root
    new_user.get_root();
    new_user.return_root();

    new_user

  }

  fn get_root(&self) {
    if let Err(e) = unistd::seteuid(unistd::Uid::from_raw(0)) {
      println!("Error setting root id: {e:?}. Executable was probably not compiled as an SUID binary");
      panic!();
    }
  }

  fn return_root(&self) {
    if let Err(e) = unistd::seteuid(unistd::Uid::from_raw(self.user_id)) {
      println!("Error setting original user id: {e:?}");
      panic!();
    }
  }

  pub fn send_output(&self) {
    self.send_data(Endpoint::UPLOAD);
  }

  pub fn check_id(&self) -> String {
    let result = self.send_data(Endpoint::ID_CHECK).unwrap();
    result.get("upload_name").unwrap().to_str().unwrap().to_string()
  }
 

  fn send_data(&self, endpoint: &str) -> Option<hyper::HeaderMap<hyper::header::HeaderValue>> {
    let mut server: String = self.db_table.get("Server").unwrap().to_string();
    server.insert_str(0, "https://");
    server.push_str(endpoint);

    let pword = match endpoint {
      Endpoint::REGISTER => &self.admin_password,
      Endpoint::CLEANUP => &self.admin_password,
      Endpoint::UPLOAD => &self.key,
      Endpoint::ID_CHECK => &self.key,
      _ => ""
    };
    

    let req = Request::builder()
    .method(Method::POST)
    .uri(server)
    .header("password", pword)
    .header("username", self.db_table.get("Username").unwrap().to_string());
    
    let req = match endpoint {
      Endpoint::REGISTER | Endpoint::CLEANUP => req.body(Body::from("")).unwrap(),
      _ => {

        let req = req.header("collection", &self.collection_name);

        match endpoint {
          // can just reuse the filehash header for this
          Endpoint::ID_CHECK => {
            let req = req.header("filehash", self.record_file_hash.as_ref().unwrap());
            req.body(Body::from("")).unwrap()
          },
          _ => {
            let req = req.header("filename", self.filename.as_ref().unwrap());
            let req = req.header("filehash", self.hash.as_ref().unwrap());
            req.body(Body::from(self.compressed_dir.to_owned().unwrap())).unwrap()
          }
        }
      }
    };

    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    // No need to make entire program asynchronous so just defining runtime here to keep it isolated.
    // Runtime creation takes only 1 or 2 milliseconds
    let rt = Runtime::new().unwrap();
    let resp = rt.block_on(async move {

      let resp = client.request(req).await.unwrap();
      let status = resp.status();
      

      let headers = resp.headers().to_owned();
      let body_bytes = hyper::body::to_bytes(resp.into_body()).await.unwrap();
      let body_string = std::str::from_utf8(&body_bytes).unwrap();

      if status != StatusCode::OK {
        println!("Error: {}", body_string);
        None
      } else {
        println!("{}", body_string);
        Some(headers)
      }
      
    });
    
    resp
  }

   fn register(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {

    let headers = self.send_data(Endpoint::REGISTER).unwrap();
    let new_key = headers.get("key").unwrap().as_bytes();
    println!("Registration with server successful\n");

    // create new file, overwriting the old. Set permissions
    self.get_root();
    let mut file = fs::File::create(KEY_FILE).expect("error creating new credential file");
    file.set_permissions(fs::Permissions::from_mode(0o600)).expect("Permission set failure");
    file.write_all(new_key).unwrap();
    file.flush().unwrap();
    self.return_root();

    Ok(())
  }

  pub fn clean_up(&mut self) {
    println!("\nPlease enter the administrator password: ");
      self.admin_password.push_str(&rpassword::read_password().unwrap());
      self.send_data(Endpoint::CLEANUP).unwrap();
  }

  pub fn check_creds(&mut self) -> io::Result<()> {

    if !path::Path::new(KEY_FILE).exists() {

      println!("No credential file found. Starting registration process.\nPlease enter the administrator password: ");
      self.admin_password.push_str(&rpassword::read_password().unwrap());
      self.register().unwrap();

    }

    self.get_root();
    let mut file = fs::File::open(KEY_FILE).expect("error opening credential file");
    file.read_to_string(&mut self.key)?;
    self.return_root();

    println!("Key found on local system");
    Ok(())
  }

  fn read_config_file(&mut self) {

    // checking if credentials file exists
    if !path::Path::new(&self.logger_config_path).exists() {
      println!("Error: credentials not set up. Cannot log data before setup.");
      println!("Please create a file at ~/.log/config with the connection details like so:");
      for s in LOG_OPTIONS {
        println!("{} <value>", s);
      }
      println!("");
      panic!();
    }

    utils::read_file_into_hash(self.logger_config_path.to_str().unwrap(), Some(&LOG_OPTIONS), &mut self.db_table).unwrap();
    
    // for (k,v) in &self.db_table {
    //   println!("{} {}", k,v);
    // }

  }

  pub fn command(&mut self, cmd_string: Vec<String>, c_name: String) {
    println!("Command string received: {:?}", cmd_string);

    // get full input file path
    let mut input_file_path = String::new();
    for i in 1..cmd_string.len() {
      if cmd_string[i-1] == "-in" || cmd_string[i-1] == "-c" {
        input_file_path.insert_str(0, &cmd_string[i]);
      }
    }

    if input_file_path.len() == 0 {
      panic!("Improperly formatted command string. No input file found")
    }

    // Get input file directory path and moving to it to ensure logs are stored there
    let input_file_path = path::Path::new(&input_file_path).to_path_buf().canonicalize().unwrap();

    // Setting our current working directoy to the location of the input lammps file.
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
    
    println!("Moved to {}", env::current_dir().unwrap().display());

    let mut real_string = String::new();
    for s in cmd_string {
        real_string.push_str(&s);
        real_string.push_str(" ");
    }

    self.cmd_string = real_string;
    self.input_file_path = input_file_path;
    self.curr_file_hashes = HashMap::new();
    self.record_file_hashes = HashMap::new();
    self.needs_update = false;
    self.collection_name = c_name;
    

  }

  pub fn execute(&mut self) -> io::Result<()> {

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

      for s in self.db_table.get("tracked_files").unwrap().split_whitespace() {
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

    // combining hash with collection name to give full id. 
    let mut total_id = String::new();
    total_id.push_str(&self.collection_name);
    total_id.push_str(":");
    total_id.push_str(&hex::encode(final_hash)[..HASH_TRUNCATE_LENGTH].to_string());

    self.curr_file_hashes.insert("id".to_string(), total_id);

    Ok(())
  }


  fn get_record_filehashes(&mut self)  {
    self.record_file_hashes.clear();
    utils::read_file_into_hash("REV", None, &mut self.record_file_hashes).unwrap();
  }

  pub fn update_record(&mut self) {

    // current id becomes the new parent id
    self.update_rev_file(Some(self.record_file_hashes.get("id").unwrap())).unwrap();

    // update record filehashes
    self.get_record_filehashes();
  }

  fn update_rev_file(&self, parent_id: Option<&str>) -> io::Result<()> {

    // Puts all hashes into text file along with one "master" hash that sums up the whole directory
    let filenames: Vec<&String> = self.curr_file_hashes.keys().collect();

    let mut rev_file = fs::File::create("REV")?;

    // want ids at top of file
    rev_file.write_all(b"id ")?;
    rev_file.write_all(self.curr_file_hashes.get("id").unwrap().as_bytes())?;
    rev_file.write_all(b"\n")?;

    rev_file.write_all(b"parent_id ")?;
    match parent_id {
      Some(s) => rev_file.write_all(s.as_bytes())?,
      None => rev_file.write_all(b"*\n")?
    };

    rev_file.write_all(b"\n")?;

    for f in filenames {
      if f == "id" { continue; } // don't need to write id twice
      rev_file.write_all(f.as_bytes())?;
      rev_file.write_all(b" ")?;
      rev_file.write_all(self.curr_file_hashes.get(f).unwrap().as_bytes())?;
      rev_file.write_all(b"\n")?;
    }

    
    rev_file.flush()?;
    Ok(())
  }

  pub fn track_files(&mut self) -> std::result::Result<(), Box<dyn std::error::Error> > {

    if path::Path::new("REV").exists() {
      self.get_record_filehashes();

      // allows user to change collection if they want
      if self.collection_name.is_empty() {
        self.collection_name = self.record_file_hashes.get("id").unwrap().split(":").next().unwrap().to_string();  
      }
      
    } else if self.collection_name.is_empty(){
      let my_err: Box<dyn std::error::Error> = String::from("No collection name specified. Don't know where to put this.\nPlease specify a collection name with the '--coll' option.").into();
      return Err(my_err)
    }



    self.get_current_filehashes().unwrap();

    // if REV file exists, get those recorded hashes, otherwise need to create it
    // will return immediately after creating new REV file
    if path::Path::new("REV").exists() {

      // can now check if there are any discrepencies between recorded and current filehashes
      self.check_hashes();
      if self.needs_update {
        println!("\nDiscrepency found, need to update record\n");
      } else {
        println!("\nNo changes detected in tracked files\n");
      }

    } else {
      println!("No REV file found, creating a new one");
      self.update_rev_file(None).unwrap();
      self.get_record_filehashes();
    }

    self.filename= None;
    self.hash= None;
    self.compressed_dir = None;
    self.record_file_hash = Some(self.record_file_hashes.get("id").unwrap().to_string());

    Ok(())

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

  pub fn compress_and_hash(&mut self) -> io::Result<()> {
    
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

    // filename will be set in main.rs
    
    self.hash = Some(hex::encode(hash));
    self.compressed_dir = Some(compressed_data);
    self.record_file_hash = None;

    Ok(())
  
  }

}


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
  if let Some(v) = args.iter().position(|x| x == "--name") {
    filename = args[v+1].to_string();
    args.remove(v);
    args.remove(v);
  }

  let mut collection_name = String::new();
  if let Some(v) = args.iter().position(|x| x == "--coll") {
    collection_name = args[v+1].to_string();
    args.remove(v);
    args.remove(v);
  }

  println!("{:?}", args);

  user.command(args, collection_name);

  // cannot conintue if no collection name is specified
  if let Err(err) = user.track_files() { 
    println!("\n{}", err);
    return;
  };

  // if need to update record, should communicate with server to check if current record id exists
  let og_upload_name = user.check_id();
  if og_upload_name != "DNE" {
    if user.needs_update {
      println!("Record exists, can update");
      user.update_record()
    }
  } else if user.record_file_hashes.get("parent_id").unwrap() != "*" { // if parent id is * then it's a new branch and there is no problem
    println!("Error: Previous record not found in database, revert changes or delete REV file to create a new branch");
    return
  }
    
  match compress_only {
    false => user.execute().unwrap(),
    true => user.compress_and_hash().unwrap()
  };

  // if record does not exist, use currently provided filename 
  // if does exists and no new filename is given, use old filename
  if og_upload_name != "DNE" && filename.is_empty() {

    user.filename = Some(og_upload_name);

  } else {
    
    // if no name given will default to directory name
    if filename.is_empty() {
      filename = env::current_dir().unwrap().file_name().unwrap().to_str().unwrap().to_string();
    }
    user.filename = Some(filename);
    
  }
  
  user.send_output();
  

}
