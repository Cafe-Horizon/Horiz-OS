use std::fs;
use std::io;
use std::path::Path;

pub fn rmdir(args: Vec<String>) -> io::Result<()> {
    if args.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "使用法: rmdir <directory>..."));
    }

    for dir in args {
        let path = Path::new(&dir);
        fs::remove_dir(path)?;
    }
    Ok(())
}
