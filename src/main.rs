#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    println!("$ ");
    io::stdout().flush().unwrap();
    let stdin = io::read_to_string(io::stdin());
    if let Ok(message) = stdin {
        println!("{}: command not found", message);
    }
}
