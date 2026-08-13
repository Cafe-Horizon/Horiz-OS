use std::fs;
use std::io;
use std::path::Path;

pub fn mv(args: Vec<String>) -> io::Result<()> {
    if args.len() < 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "使用法: mv <src>... <dest>"));
    }

    let mut sources = args;
    let dest_str = sources.pop().unwrap();
    let dest = Path::new(&dest_str);

    if sources.len() > 1 && !dest.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "複数の項目を移動する場合、移動先はディレクトリである必要があります。"));
    }

    for src_str in sources {
        let src = Path::new(&src_str);
        if !src.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("{}: 移動元が存在しません。", src_str)));
        }

        let target_path = if dest.is_dir() {
            dest.join(src.file_name().unwrap_or_default())
        } else {
            dest.to_path_buf()
        };

        if let Err(_) = fs::rename(src, &target_path) {
            // クロスデバイスリネーム失敗時のフォールバック (copy + remove)
            if src.is_dir() {
                return Err(io::Error::new(io::ErrorKind::Other, format!("{}: ディレクトリの移動に失敗しました。", src_str)));
            } else {
                fs::copy(src, &target_path)?;
                fs::remove_file(src)?;
            }
        }
    }

    Ok(())
}
