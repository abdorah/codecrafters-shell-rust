#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    print!("$ ");
    io::stdout().flush().unwrap();
    let mut message = String::new();
    io::read_to_string(message).unwrap();
    println!("{}: command not found", message);
}
