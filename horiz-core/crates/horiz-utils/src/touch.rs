use std::fs::OpenOptions;
use std::io;

pub fn touch(args: Vec<String>) -> io::Result<()> {
    if args.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "使用法: touch <file>..."));
    }

    for file_path in args {
        OpenOptions::new()
            .create(true)
            .write(true)
            .open(&file_path)?;
    }

    Ok(())
}
