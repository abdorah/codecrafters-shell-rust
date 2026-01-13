use codecrafters_shell::Shell;
fn main() {
    let mut shell = Shell::new();
    if let Err(e) = shell.run() {
        eprintln!("Shell error: {}", e);
        std::process::exit(1);
    }
}
