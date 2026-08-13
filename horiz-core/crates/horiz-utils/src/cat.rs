use std::fs;
use std::io::{self, Read, Write};

pub fn cat(files: Vec<String>) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let mut buffer = [0; 1024];

    if files.is_empty() {
        let mut stdin = io::stdin();
        loop {
            let n = stdin.read(&mut buffer)?;
            if n == 0 { break; }
            handle.write_all(&buffer[..n])?;
        }
    } else {
        for file in files {
            let mut f = fs::File::open(file)?;
            loop {
                let n = f.read(&mut buffer)?;
                if n == 0 { break; }
                handle.write_all(&buffer[..n])?;
            }
        }
    }
    handle.flush()?;
    Ok(())
}
