use std::fs;
use std::io;
use std::path::Path;

pub fn rm(args: Vec<String>) -> io::Result<()> {
    if args.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "使用法: rm [-r|-R] [-f] <file|dir>..."));
    }

    let mut recursive = false;
    let mut force = false;
    let mut targets = Vec::new();

    for arg in args {
        if arg == "-r" || arg == "-R" {
            recursive = true;
        } else if arg == "-f" {
            force = true;
        } else if !arg.starts_with('-') {
            targets.push(arg);
        }
    }

    if targets.is_empty() {
        if force { return Ok(()); }
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "削除対象が指定されていません。"));
    }

    for target in targets {
        let path = Path::new(&target);
        if !path.exists() {
            if force { continue; }
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("{}: ファイルまたはディレクトリが存在しません。", target)));
        }

        let res = if path.is_dir() {
            if recursive {
                fs::remove_dir_all(path)
            } else {
                Err(io::Error::new(io::ErrorKind::PermissionDenied, format!("{}: ディレクトリです。-r オプションを指定してください。", target)))
            }
        } else {
            fs::remove_file(path)
        };

        if let Err(e) = res {
            if !force { return Err(e); }
        }
    }

    Ok(())
}
