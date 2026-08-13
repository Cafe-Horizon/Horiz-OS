use std::fs;
use std::io;

pub fn ls(path: &str) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.is_file() {
        println!("{}", path);
        return Ok(());
    }

    let entries = fs::read_dir(path)?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        
        // ドットファイル（隠しファイル）を除外
        if name_str.starts_with('.') {
            continue;
        }
        
        print!("{}  ", name_str);
    }
    println!();
    Ok(())
}
