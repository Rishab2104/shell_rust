use std::env;
use std::io::{self, stdin, stdout, Write};
use std::path::Path;
use std::process::{Command, Stdio};

fn main() {
    loop {
        print!("> ");
        // Ensure the prompt is printed before taking input
        stdout().flush().unwrap();

        let mut input = String::new();
        
        // BUG 5 FIXED: Check for EOF (Ctrl+D) to prevent infinite loops/panics
        let bytes_read = stdin().read_line(&mut input).unwrap();
        if bytes_read == 0 {
            println!(); // Print a newline so the terminal prompt looks clean
            return;
        }

        let input = input.trim();
        
        // BUG 1 FIXED: Skip empty inputs (e.g., user just pressing Enter)
        if input.is_empty() {
            continue;
        }

        let mut commands = input.split(" | ").peekable();
        
        // BUG 3 FIXED: Store ALL child processes here to wait on them at the end (No Zombies)
        let mut children = Vec::new();
        
        // We only need to carry the output stream forward to the next command
        let mut previous_stdout = None;

        while let Some(command) = commands.next() {
            let mut parts = command.trim().split_whitespace();
            
            // BUG 2 FIXED: Gracefully handle missing commands (e.g., dangling pipe: "ls | ")
            let cmd = match parts.next() {
                Some(c) => c,
                None => {
                    eprintln!("Error: Expected command after pipe.");
                    break; // Abort the rest of the pipeline
                }
            };

            let args = parts;

            match cmd {
                "cd" => {
                    let new_dir = args.peekable().peek().map_or("/", |x| *x);
                    let root = Path::new(new_dir);
                    if let Err(e) = env::set_current_dir(&root) {
                        eprintln!("cd: {}", e);
                    }
                    // Built-in commands don't pipe stdout in this basic implementation
                    previous_stdout = None;
                }
                "exit" => return,
                _ => {
                    // Stdin: from the previous command, or the keyboard if it's the first
                    let stdin = previous_stdout.map_or(
                        Stdio::inherit(),
                        |stdout| Stdio::from(stdout),
                    );

                    // Stdout: to a pipe if there is another command, or the screen if it's the last
                    let stdout = if commands.peek().is_some() {
                        Stdio::piped()
                    } else {
                        Stdio::inherit()
                    };

                    let output = Command::new(cmd)
                        .args(args)
                        .stdin(stdin)
                        .stdout(stdout)
                        .spawn();

                    match output {
                        Ok(mut child) => {
                            // Extract stdout to pass to the NEXT command in the pipeline
                            previous_stdout = child.stdout.take();
                            
                            // Save the child process so we can wait() on it later
                            children.push(child);
                        }
                        Err(e) => {
                            eprintln!("Error executing {}: {}", cmd, e);
                            // BUG 4 FIXED: Abort the pipeline so subsequent commands 
                            // don't execute and mistakenly read from the keyboard.
                            break; 
                        }
                    }
                }
            }
        }

        // Wait for ALL child processes to finish before showing the prompt again.
        for mut child in children {
            let _ = child.wait();
        }
    }
}