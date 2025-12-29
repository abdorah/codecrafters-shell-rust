#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    print!("$ ");
    io::stdout().flush().unwrap();
    let mut message = String::new();
    let stdin = io::read_to_string(message);
    if let Ok(message) = stdin {
        println!("{}: command not found", message);
    }
}
