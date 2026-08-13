use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

pub fn tail(args: Vec<String>) -> io::Result<()> {
    let mut num_lines: usize = 10;
    let mut files = Vec::new();
    let mut i = 0;

    while i < args.len() {
        if args[i] == "-n" {
            if i + 1 < args.len() {
                num_lines = args[i + 1].parse().unwrap_or(10);
                i += 2;
                continue;
            }
        } else if args[i].starts_with("-n") {
            num_lines = args[i][2..].parse().unwrap_or(10);
        } else {
            files.push(args[i].clone());
        }
        i += 1;
    }

    let print_tail = |reader: &mut dyn BufRead| -> io::Result<()> {
        let mut buffer = VecDeque::new();
        for line in reader.lines() {
            let l = line?;
            if buffer.len() >= num_lines {
                buffer.pop_front();
            }
            buffer.push_back(l);
        }
        for l in buffer {
            println!("{}", l);
        }
        Ok(())
    };

    if files.is_empty() {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        print_tail(&mut handle)?;
    } else {
        let multi = files.len() > 1;
        for file_path in files {
            if multi { println!("==> {} <==", file_path); }
            let file = File::open(&file_path)?;
            let mut reader = BufReader::new(file);
            print_tail(&mut reader)?;
        }
    }

    Ok(())
}
