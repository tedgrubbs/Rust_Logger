pub struct Command {
  
  cmd_string: String,
  
}

impl Command {

  pub fn command(cmd_string: String) -> Command {
    println!("Command string received");

    Command {
      cmd_string,
    }
    
    
  }

  pub fn execute(&self) {
    
  }

}