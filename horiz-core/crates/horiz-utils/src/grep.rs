use std::fs::File;
use std::io::{self, BufRead, BufReader};

pub fn grep(args: Vec<String>) -> io::Result<()> {
    if args.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "使用法: grep [-i] [-n] <pattern> [file...]"));
    }

    let mut ignore_case = false;
    let mut show_line_num = false;
    let mut pattern = String::new();
    let mut files = Vec::new();

    for arg in args {
        if arg == "-i" {
            ignore_case = true;
        } else if arg == "-n" {
            show_line_num = true;
        } else if pattern.is_empty() {
            pattern = arg;
        } else {
            files.push(arg);
        }
    }

    if pattern.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "検索パターンが指定されていません。"));
    }

    let target_pattern = if ignore_case { pattern.to_lowercase() } else { pattern.clone() };

    let process_reader = |reader: &mut dyn BufRead, filename: Option<&str>| -> io::Result<()> {
        let mut line_num = 0;
        for line in reader.lines() {
            line_num += 1;
            let line_str = line?;
            let search_str = if ignore_case { line_str.to_lowercase() } else { line_str.clone() };

            if search_str.contains(&target_pattern) {
                let mut prefix = String::new();
                if let Some(name) = filename {
                    prefix.push_str(&format!("{}:", name));
                }
                if show_line_num {
                    prefix.push_str(&format!("{}:", line_num));
                }
                println!("{}{}", prefix, line_str);
            }
        }
        Ok(())
    };

    if files.is_empty() {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        process_reader(&mut handle, None)?;
    } else {
        let multi = files.len() > 1;
        for file_path in files {
            let file = File::open(&file_path)?;
            let mut reader = BufReader::new(file);
            let name_tag = if multi { Some(file_path.as_str()) } else { None };
            process_reader(&mut reader, name_tag)?;
        }
    }

    Ok(())
}
