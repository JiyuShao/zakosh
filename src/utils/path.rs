use std::borrow::Cow;
use std::env;
use std::fs::read_dir;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;

use log::error;

pub fn basename(path: &str) -> Cow<'_, str> {
    let mut pieces = path.rsplit('/');
    match pieces.next() {
        Some(p) => p.into(),
        None => path.into(),
    }
}

pub fn find_file_in_path(filename: &str, exec: bool) -> String {
    let env_path = match env::var("PATH") {
        Ok(x) => x,
        Err(e) => {
            error!("zako: error with env PATH: {:?}", e);
            return String::new();
        }
    };
    let vec_path: Vec<&str> = env_path.split(':').collect();
    for p in &vec_path {
        match read_dir(p) {
            Ok(list) => {
                for entry in list.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name != filename {
                            continue;
                        }

                        if exec {
                            let _mode = match entry.metadata() {
                                Ok(x) => x,
                                Err(e) => {
                                    error!("zako: metadata error: {:?}", e);
                                    continue;
                                }
                            };
                            let mode = _mode.permissions().mode();
                            if mode & 0o111 == 0 {
                                // not binary
                                continue;
                            }
                        }

                        return entry.path().to_string_lossy().to_string();
                    }
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    continue;
                }
                error!("zako: fs read_dir error: {}: {}", p, e);
            }
        }
    }
    String::new()
}

pub fn current_dir() -> String {
    let _current_dir = match env::current_dir() {
        Ok(x) => x,
        Err(e) => {
            error!("zako: PROMPT: env current_dir error: {}", e);
            return String::new();
        }
    };
    let current_dir = match _current_dir.to_str() {
        Some(x) => x,
        None => {
            error!("zako: PROMPT: to_str error");
            return String::new();
        }
    };

    current_dir.to_string()
}
