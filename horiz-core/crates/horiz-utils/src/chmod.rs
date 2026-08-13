use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// 8進数形式の指定に基づいてファイル権限を変更 (Zero-Dependency)
pub fn chmod(args: Vec<String>) -> io::Result<()> {
    if args.len() < 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Usage: chmod <octal_mode> <file1> [<file2> ...]"));
    }

    let mode_str = &args[0];
    let mode = u32::from_str_radix(mode_str, 8).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("Invalid octal mode '{}'", mode_str))
    })?;

    for file_path in args.iter().skip(1) {
        let permissions = fs::metadata(file_path)?.permissions();
        #[cfg(unix)]
        {
            let mut permissions = permissions;
            permissions.set_mode(mode);
            fs::set_permissions(file_path, permissions)?;
        }
        #[cfg(not(unix))]
        {
            let _ = (permissions, mode);
        }
    }

    Ok(())
}
