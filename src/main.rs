//! Simple Shell - Main Entry Point
//! 
//! A cross-platform shell implementation that provides basic shell functionality
//! including command execution, built-in commands, and tab completion.
//! 
//! This binary creates and runs a Shell instance, handling any errors that occur
//! during shell operation.

use codecrafters_shell::Shell;

/// Main entry point for the shell application
/// 
/// Creates a new Shell instance and runs the interactive shell loop.
/// If any errors occur during shell operation, they are printed to stderr
/// and the program exits with code 1.
fn main() {
    let mut shell = Shell::new();
    if let Err(e) = shell.run() {
        eprintln!("Shell error: {}", e);
        std::process::exit(1);
    }
}
