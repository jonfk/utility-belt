- [ ] 

# Hashing loads entire files into memory.

fs::read() for hashing will spike memory with large files and rayon parallelism. Switch to streaming:

```
use std::{fs::File, io::{Read, BufReader}};
fn calculate_file_hash_streaming(path: &Utf8PathBuf) -> Result<String, AppError> {
    let mut hasher = Sha256::new();
    let mut f = BufReader::new(File::open(path).change_context(AppError::IO)?);
    let mut buf = [0u8; 1024 * 1024];
    loop {
        let n = f.read(&mut buf).change_context(AppError::IO)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
```

(Wire that into both FileScanner and FileCopier.)
