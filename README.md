# Simple Shell

A cross-platform shell implementation in Rust that provides basic shell functionality including command execution, built-in commands, tab completion, and I/O redirection.

## Features

- **Built-in Commands**: `echo`, `pwd`, `cd`, `type`, `exit`
- **External Command Execution**: Run any executable in your PATH
- **Tab Completion**: Auto-complete commands and show suggestions with double-tab
- **I/O Redirection**: Support for `>`, `>>`, `1>`, `2>` redirection operators
- **Cross-Platform**: Works on Windows, macOS, and Linux
- **Line Editing**: Full cursor movement, backspace, delete with arrow keys
- **Control Keys**: Support for Ctrl+C (interrupt), Ctrl+D (EOF), Ctrl+A/E (home/end)

## Quick Start

### Building

```bash
# Clone the repository
git clone <repository-url>
cd shell

# Build the project
cargo build --release

# Run the shell
cargo run
```

### Usage

Once running, you can use the shell like any other shell:

```bash
$ echo "Hello, World!"
Hello, World!

$ pwd
/home/user/shell

$ cd /tmp
$ pwd
/tmp

$ type echo
echo is a shell builtin

$ type ls
ls is /usr/bin/ls

$ echo "Hello" > output.txt
$ cat output.txt
Hello

$ exit 0
```

## Architecture

The shell is organized into several modules:

### Core Components

- **`Shell`** (`src/lib.rs`): Main shell implementation with command processing
- **`handle`** (`src/handle/`): Input handling and line editing
- **`terminal`** (`src/terminal/`): Terminal raw mode control

### Module Structure

```
src/
├── lib.rs              # Main shell implementation
├── main.rs             # Binary entry point
├── handle/             # Input handling module
│   ├── mod.rs          # Module declarations
│   ├── constants.rs    # Key definitions
│   ├── lib.rs          # Line editor implementation
│   ├── unix.rs         # Unix key reading
│   └── windows.rs      # Windows key reading
└── terminal/           # Terminal control module
    ├── mod.rs          # Module declarations
    ├── unix.rs         # Unix raw mode
    └── windows.rs      # Windows raw mode
```

## Built-in Commands

### `echo [args...]`
Prints arguments to stdout, separated by spaces.

```bash
$ echo Hello World
Hello World
```

### `pwd`
Prints the current working directory.

```bash
$ pwd
/home/user
```

### `cd [directory]`
Changes the current directory.

```bash
$ cd /tmp          # Change to /tmp
$ cd               # Change to home directory
$ cd ~             # Change to home directory
$ cd ~/Documents   # Change to ~/Documents
```

### `type <command>`
Shows information about a command (builtin vs external).

```bash
$ type echo
echo is a shell builtin

$ type ls
ls is /usr/bin/ls

$ type nonexistent
nonexistent: not found
```

### `exit [code]`
Exits the shell with optional exit code (default: 0).

```bash
$ exit
$ exit 1
```

## I/O Redirection

The shell supports several redirection operators:

### Output Redirection
```bash
$ echo "Hello" > file.txt        # Redirect stdout to file (overwrite)
$ echo "World" >> file.txt       # Redirect stdout to file (append)
$ echo "Hello" 1> file.txt       # Explicit stdout redirection
```

### Error Redirection
```bash
$ command 2> errors.txt          # Redirect stderr to file
$ command 2>> errors.txt         # Redirect stderr to file (append)
```

## Tab Completion

The shell provides intelligent tab completion:

- **Single Tab**: Auto-completes if there's only one match, or completes the common prefix
- **Double Tab**: Shows all possible completions

```bash
$ ec<TAB>           # Completes to "echo "
$ l<TAB><TAB>       # Shows: ls ln less ...
```

## Line Editing

Full line editing support with:

- **Arrow Keys**: Move cursor left/right
- **Home/End**: Jump to beginning/end of line
- **Ctrl+A/E**: Alternative home/end
- **Backspace/Delete**: Character deletion
- **Ctrl+C**: Interrupt current input
- **Ctrl+D**: EOF (exit on empty line)

## Cross-Platform Support

The shell works on multiple platforms through conditional compilation:

### Unix-like Systems (Linux, macOS)
- Uses `termios` for terminal control
- Uses Unix file permissions for executable detection
- Supports standard Unix path conventions

### Windows
- Uses Windows Console API for terminal control
- Uses file extensions for executable detection (.exe, .bat, .cmd, etc.)
- Supports Windows path conventions

## Development

### Project Structure

The project follows Rust best practices with clear module separation:

- **Library crate** (`src/lib.rs`): Contains all the shell logic
- **Binary crate** (`src/main.rs`): Simple entry point that uses the library
- **Modular design**: Separate modules for different concerns
- **Platform abstraction**: Unified interfaces with platform-specific implementations

### Building and Testing

```bash
# Check for compilation errors
cargo check

# Build debug version
cargo build

# Build release version
cargo build --release

# Run with debug output
RUST_LOG=debug cargo run

# Generate documentation
cargo doc --open
```

### Code Organization

The codebase is organized around these principles:

1. **Separation of Concerns**: Input handling, terminal control, and shell logic are separate
2. **Cross-Platform**: Platform-specific code is isolated in separate modules
3. **Clean APIs**: Public interfaces are minimal and well-documented
4. **Error Handling**: Proper error propagation using `Result` types

## Documentation

Comprehensive documentation is available:

```bash
# Generate and open HTML documentation
cargo doc --open
```

The documentation includes:
- API documentation for all public functions
- Module-level documentation explaining architecture
- Examples for key functionality
- Cross-references between related components

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes with appropriate tests
4. Ensure documentation is updated
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built as part of the CodeCrafters Shell challenge
- Inspired by POSIX shell standards
- Uses modern Rust practices and idioms