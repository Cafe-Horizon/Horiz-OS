use horiz_utils;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(0)
        .and_then(|path| Path::new(path).file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    let (target_cmd, arg_offset) = if cmd == "horiz-utils" {
        (args.get(1).map(|s| s.as_str()).unwrap_or(""), 2)
    } else {
        (cmd.as_str(), 1)
    };

    let remaining_args = if args.len() >= arg_offset {
        args[arg_offset..].to_vec()
    } else {
        Vec::new()
    };

    match target_cmd {
        "ls" => {
            let path = remaining_args.get(0).map(|s| s.as_str()).unwrap_or(".");
            if let Err(e) = horiz_utils::ls(path) {
                eprintln!("ls: {}", e);
            }
        }
        "cat" => {
            if !remaining_args.is_empty() {
                if let Err(e) = horiz_utils::cat(remaining_args) {
                    eprintln!("cat: {}", e);
                }
            }
        }
        "echo" => {
            horiz_utils::echo(remaining_args);
        }
        "chmod" => {
            if let Err(e) = horiz_utils::chmod(remaining_args) {
                eprintln!("chmod: {}", e);
            }
        }
        "date" => {
            if let Err(e) = horiz_utils::date() {
                eprintln!("date: {}", e);
            }
        }
        _ => eprintln!("Unknown utility: {}", target_cmd),
    }
}
