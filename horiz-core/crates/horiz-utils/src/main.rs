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

    let res = match target_cmd {
        "ls" => {
            let path = remaining_args.get(0).map(|s| s.as_str()).unwrap_or(".");
            horiz_utils::ls(path)
        }
        "cat" => {
            if remaining_args.is_empty() {
                Ok(())
            } else {
                horiz_utils::cat(remaining_args)
            }
        }
        "echo" => {
            horiz_utils::echo(remaining_args);
            Ok(())
        }
        "chmod" => horiz_utils::chmod(remaining_args),
        "date" => horiz_utils::date(),
        "mkdir" => horiz_utils::mkdir(remaining_args),
        "rmdir" => horiz_utils::rmdir(remaining_args),
        "rm" => horiz_utils::rm(remaining_args),
        "cp" => horiz_utils::cp(remaining_args),
        "mv" => horiz_utils::mv(remaining_args),
        "touch" => horiz_utils::touch(remaining_args),
        "ps" => horiz_utils::ps(remaining_args),
        "kill" => horiz_utils::kill(remaining_args),
        "grep" => horiz_utils::grep(remaining_args),
        "head" => horiz_utils::head(remaining_args),
        "tail" => horiz_utils::tail(remaining_args),
        "wc" => horiz_utils::wc(remaining_args),
        _ => {
            eprintln!("Unknown utility: {}", target_cmd);
            Ok(())
        }
    };

    if let Err(e) = res {
        eprintln!("{}: {}", target_cmd, e);
    }
}
