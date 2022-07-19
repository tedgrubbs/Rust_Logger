use std::env;
use std::path;
use nix::unistd;

const LOGGER_CREDENTIALS_FILE: &str = "/etc/Rust_Logger_Credentials";

fn main() {

    let args: Vec<String> = env::args().collect();

    if args.len() == 1 || args[1] == "-h" {
        println!("Usage:");
        println!("log <command> // logs command and logs to server");
        println!("log -user <username> // stores remote server username");
        println!("log -password <password> // stores remote server password");
        println!("log -serverip <serverip> // stores remote server IP address");
        println!("log -privatekey <privatekey> // stores remote server private key");
        return
    }

    if !path::Path::new(LOGGER_CREDENTIALS_FILE).exists() {
        println!("Error: credentials not set up. Cannot log data before setup.");
        return
    }

    let og_id: unistd::Uid = unistd::Uid::current();

    if let Err(e) = unistd::setuid(unistd::Uid::from_raw(0)) {
        println!("Error setting user id: {e:?}");
    }
    // match unistd::setuid(id) {
    //     Ok(_) => (),
    //     Err(e) => println!("Error setting user id: {e:?}"),
    // }
    println!("User id:{}", unistd::Uid::effective());
    println!("Original id:{}", og_id);

    println!("Passed error checks...");

}
