use std::fs;
use std::io;
use std::path::Path;

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

pub fn cp(args: Vec<String>) -> io::Result<()> {
    if args.len() < 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "使用法: cp [-r|-R] <src>... <dest>"));
    }

    let mut recursive = false;
    let mut sources = Vec::new();

    for arg in args {
        if arg == "-r" || arg == "-R" {
            recursive = true;
        } else {
            sources.push(arg);
        }
    }

    if sources.len() < 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "コピー元およびコピー先を指定してください。"));
    }

    let dest_str = sources.pop().unwrap();
    let dest = Path::new(&dest_str);

    if sources.len() > 1 && !dest.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "複数のファイルをコピーする場合、コピー先はディレクトリである必要があります。"));
    }

    for src_str in sources {
        let src = Path::new(&src_str);
        if !src.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("{}: コピー元が存在しません。", src_str)));
        }

        let target_path = if dest.is_dir() {
            dest.join(src.file_name().unwrap_or_default())
        } else {
            dest.to_path_buf()
        };

        if src.is_dir() {
            if recursive {
                copy_dir_all(src, &target_path)?;
            } else {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, format!("{}: ディレクトリです。-r オプションを指定してください。", src_str)));
            }
        } else {
            fs::copy(src, target_path)?;
        }
    }

    Ok(())
}
