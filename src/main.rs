#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    println!("$ ");
    let stdin = io::read_to_string(io::stdin())?;
    println!("{}: command not found", stdin);
    io::stdout().flush().unwrap();
}
