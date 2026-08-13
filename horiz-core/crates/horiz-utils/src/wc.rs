use std::fs::File;
use std::io::{self, Read};

pub fn wc(args: Vec<String>) -> io::Result<()> {
    let mut count_lines = false;
    let mut count_words = false;
    let mut count_bytes = false;
    let mut files = Vec::new();

    for arg in args {
        if arg.starts_with('-') && arg.len() > 1 {
            for c in arg[1..].chars() {
                match c {
                    'l' => count_lines = true,
                    'w' => count_words = true,
                    'c' => count_bytes = true,
                    _ => {}
                }
            }
        } else {
            files.push(arg);
        }
    }

    if !count_lines && !count_words && !count_bytes {
        count_lines = true;
        count_words = true;
        count_bytes = true;
    }

    let process_content = |data: &[u8], name: Option<&str>| {
        let lines = data.split(|&b| b == b'\n').count() - if data.ends_with(b"\n") { 1 } else { 0 };
        let text = String::from_utf8_lossy(data);
        let words = text.split_whitespace().count();
        let bytes = data.len();

        let mut out = String::new();
        if count_lines { out.push_str(&format!("{:>8} ", lines)); }
        if count_words { out.push_str(&format!("{:>8} ", words)); }
        if count_bytes { out.push_str(&format!("{:>8} ", bytes)); }
        if let Some(n) = name { out.push_str(&format!("{}", n)); }

        println!("{}", out.trim_end());
    };

    if files.is_empty() {
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer)?;
        process_content(&buffer, None);
    } else {
        for file_path in files {
            let mut file = File::open(&file_path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            process_content(&buffer, Some(&file_path));
        }
    }

    Ok(())
}
