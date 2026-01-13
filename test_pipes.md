# Pipe Testing Guide

## Test Cases for Pipe Implementation

### Basic Pipe Tests
1. `echo "hello world" | cat` - Should output "hello world"
2. `echo -e "line1\nline2\nline3" | grep line2` - Should output "line2"
3. `ls | head -5` - Should show first 5 files

### Multi-stage Pipes
1. `echo -e "c\nb\na" | sort | head -1` - Should output "a"
2. `ps | grep shell | wc -l` - Should count shell processes

### Pipes with Redirection
1. `echo "test" | cat > output.txt` - Should create file with "test"
2. `ls | grep .txt 2> errors.txt` - Should redirect errors to file

### Error Cases
1. `nonexistent | cat` - Should show "command not found"
2. `echo test | nonexistent` - Should show error for second command
3. `echo | cd /tmp` - Should show builtin not supported in pipeline

## Manual Testing

To test the pipe implementation:

1. Build the shell: `cargo build`
2. Run the shell: `cargo run`
3. Try the test cases above
4. Verify output matches expectations

## Expected Behavior

- Simple pipes should work between external commands
- Built-in commands work only as the last command in a pipeline
- Error messages should be clear and helpful
- File redirections should work with pipes
- Multiple pipes in sequence should work correctly