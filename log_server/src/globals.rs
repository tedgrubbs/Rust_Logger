use std::collections::HashMap;
use std::{path};
use utils::utils;
// use std::time::Instant;

// Stored global configuration settings found in server config

pub struct Globals {
  pub globals: HashMap<String, String>
}

impl Globals {

  const ALLOWED_SERVER_OPTIONS: [&'static str; 7] = [
      "server_port",
      "cert_path", 
      "key_path", 
      "data_path", 
      "database", 
      "registry",
      "tracked_files"
    ];

  pub fn new() -> Globals {

    // let runtime = Instant::now();

    let mut config_path = home::home_dir().unwrap();
    config_path.push(".log_server/config");
    
    if !path::Path::new(&config_path).exists() {
      println!("Error: credentials not set up. Cannot log data before setup.");
      println!("Please create a file at ~/.log_server/config with the connection details like so:");
      for s in Globals::ALLOWED_SERVER_OPTIONS {
        println!("{}", s);
      }
      println!("");
      panic!();
    }

    let mut globals = HashMap::with_capacity(Globals::ALLOWED_SERVER_OPTIONS.len());
    utils::read_file_into_hash(config_path.to_str().unwrap(), Some(&Globals::ALLOWED_SERVER_OPTIONS), &mut globals).unwrap();

    // println!("runtime: {}", runtime.elapsed().as_micros());

    Globals {
      globals
    }
    
  }
}