use std::fs;
use std::io;
use std::path::Path;

pub fn mkdir(args: Vec<String>) -> io::Result<()> {
    if args.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "使用法: mkdir [-p] <directory>..."));
    }

    let mut parents = false;
    let mut dirs = Vec::new();

    for arg in args {
        if arg == "-p" {
            parents = true;
        } else {
            dirs.push(arg);
        }
    }

    if dirs.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "ディレクトリパスが指定されていません。"));
    }

    for dir in dirs {
        let path = Path::new(&dir);
        if parents {
            fs::create_dir_all(path)?;
        } else {
            fs::create_dir(path)?;
        }
    }
    Ok(())
}
