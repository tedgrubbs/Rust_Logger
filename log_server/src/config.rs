use std::collections::HashMap;
use std::{path};
use utils::utils;
// use std::time::Instant;

// Stored global configuration settings found in server config

pub struct Config {
  pub config: HashMap<String, String>
}

impl Config {

  const ALLOWED_SERVER_OPTIONS: [&'static str; 5] = [
      "server_port",
      "cert_path", 
      "key_path", 
      "data_path", 
      "database"
  ];

  pub fn new() -> Config {

    // let runtime = Instant::now();

    let mut config_path = home::home_dir().unwrap();
    config_path.push(".log_server/config");
    
    if !path::Path::new(&config_path).exists() {
      println!("Error: credentials not set up. Cannot log data before setup.");
      println!("Please create a file at ~/.log_server/config with the connection details like so:");
      for s in Config::ALLOWED_SERVER_OPTIONS {
        println!("{}", s);
      }
      println!("");
      panic!();
    }

    let mut config = HashMap::with_capacity(Config::ALLOWED_SERVER_OPTIONS.len());
    utils::read_file_into_hash(config_path.to_str().unwrap(), Some(&Config::ALLOWED_SERVER_OPTIONS), &mut config).unwrap();

    // println!("runtime: {}", runtime.elapsed().as_micros());

    Config {
      config
    }
    
  }
}