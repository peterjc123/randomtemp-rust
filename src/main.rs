use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use tempfile::TempDir;
use which::which;

macro_rules! error_exit {
    ($msg:expr) => {{
        eprintln!("{}", $msg);
        process::exit(1);
    }};
}

#[warn(unused_macros)]
macro_rules! quote_arg {
    ($arg:expr) => {
        if $arg.contains(' ') {
            format!("\"{}\"", $arg)
        } else {
            format!("{}", $arg)
        }
    };
}

fn path_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}

fn dir_exists(path: &str) -> bool {
    path_exists(path) && fs::metadata(path).unwrap().is_dir()
}

fn is_absolute_path(path: &str) -> bool {
    Path::new(path).is_absolute()
}

fn find_executable_in_path_by_name(p: &PathBuf, cp: &PathBuf) -> Option<PathBuf> {
    let f = get_file_name(Some(p));
    f.and_then(|s| {
        which(s)
            .ok()
            // TODO: We should try to find another one instead of
            // skipping.
            .and_then(|np| if &np == cp { None } else { Some(np) })
    })
}

fn find_executable_in_path() -> Result<PathBuf, String> {
    let p = get_current_exe_pathbuf()?;
    find_executable_in_path_by_name(&p, &p).ok_or_else(|| {
        "Cannot find which executable to pretend, either specify \
		RANDOMTEMP_EXECUTABLE through the environmental variables \
	 	or rename the executable to another one in PATH"
            .to_string()
    })
}

fn find_executable_in_path_by_env(exec: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(&exec);
    let cp = get_current_exe_pathbuf()?;
    if is_same_file_stem(Some(&p), Some(&cp)) {
        // Stop pretending self
        return Err(String::new());
    }
    find_executable_in_path_by_name(&p, &cp)
        .or_else(|| {
            if p.extension().is_some() {
                None
            } else {
                Some(p)
            }
        })
        .ok_or_else(|| "RANDOMTEMP_EXECUTABLE points to an invalid executable".to_string())
}

fn get_current_exe_pathbuf() -> Result<PathBuf, String> {
    env::current_exe().map_err(|_| "Cannot get the current working executable".to_string())
}

fn get_current_dir_pathbuf() -> Result<PathBuf, String> {
    env::current_dir().map_err(|_| "Cannot get the current working directory".to_string())
}

#[allow(dead_code)]
fn get_current_exe() -> Result<String, String> {
    let pb = get_current_exe_pathbuf()?;
    let p = pb
        .to_str()
        .ok_or("Cannot convert the current working directory to a UTF-8 string")?;
    Ok(String::from(p))
}

fn get_current_dir() -> Result<String, String> {
    let pb = get_current_dir_pathbuf()?;
    let p = pb
        .to_str()
        .ok_or("Cannot convert the current working directory to a UTF-8 string")?;
    Ok(String::from(p))
}

fn get_file_name(pbopt: Option<&PathBuf>) -> Option<&OsStr> {
    let pb = pbopt?;
    pb.file_name()
}

fn get_file_stem(pbopt: Option<&PathBuf>) -> Option<&OsStr> {
    let pb = pbopt?;
    pb.file_stem()
}

#[allow(dead_code)]
fn is_same_file_pathbuf(pbopt1: Option<&PathBuf>, pbopt2: Option<&PathBuf>) -> bool {
    if let (Some(f1), Some(f2)) = (pbopt1, pbopt2) {
        return f1 == f2;
    }
    false
}

fn is_same_file_stem(pbopt1: Option<&PathBuf>, pbopt2: Option<&PathBuf>) -> bool {
    let fs1 = get_file_stem(pbopt1);
    let fs2 = get_file_stem(pbopt2);
    if let (Some(fs1), Some(fs2)) = (fs1, fs2) {
        return fs1 == fs2;
    }
    false
}

fn get_pretend_executable() -> String {
    env::var("RANDOMTEMP_EXECUTABLE")
        .ok()
        .and_then(|exec| {
            if is_absolute_path(&exec) {
                Some(PathBuf::from(exec))
            } else {
                find_executable_in_path_by_env(&exec)
                    .map_err(|e| {
                        if e.is_empty() {
                            e
                        } else {
                            error_exit!(e);
                        }
                    })
                    .ok()
            }
        })
        .unwrap_or_else(|| {
            find_executable_in_path().unwrap_or_else(|e| {
                error_exit!(e);
            })
        })
        .to_str()
        .map(String::from)
        .unwrap_or_else(|| {
            error_exit!("Cannot convert RANDOMTEMP_EXECUTABLE to a UTF-8 string");
        })
}

fn get_base_dir() -> String {
    env::var("RANDOMTEMP_BASEDIR")
        .map(|path| {
            if dir_exists(&path) {
                path
            } else {
                error_exit!("The directory specified in RANDOMTEMP_BASEDIR doesn't exist");
            }
        })
        .or_else(|_| get_current_dir())
        .unwrap_or_else(|e| {
            error_exit!(e);
        })
}

fn get_max_trial() -> u8 {
    const DEFAULT_MAX_TRIAL: u8 = 3;

    env::var("RANDOMTEMP_MAXTRIAL")
        .map(|val| {
            val.parse().unwrap_or_else(|_| {
                error_exit!("RANDOMTEMP_MAXTRIAL is not valid number in 0..256");
            })
        })
        .unwrap_or(DEFAULT_MAX_TRIAL)
}

#[cfg(windows)]
fn try_run_with_new_temp(cwd: &str, executable: &str) -> process::ExitStatus {
    let tmp_dir = TempDir::new_in(&cwd).unwrap_or_else(|_| {
        error_exit!("Cannot create temporary directory");
    });
    let tmp_path = &tmp_dir.path();
    if is_absolute_path(executable) {
        process::Command::new(&executable)
            .env("TEMP", tmp_path)
            .env("TMP", tmp_path)
            .args(env::args().skip(1))
            .status()
            .expect("failed to execute process")
    } else {
        // We need to quote the args here for Command Prompt
        // Below is the workground given in
        // https://internals.rust-lang.org/t/std-process-on-windows-is-escaping-raw-literals-which-causes-problems-with-chaining-commands/8163/16
        let mut args = String::new();
        args.push_str(quote_arg!(executable).as_str());
        for arg in env::args().skip(1) {
            args.push(' ');
            args.push_str(quote_arg!(arg).as_str());
        }
        let arg_name = "RANDOMTEMP_COMMANDLINE";
        process::Command::new("cmd")
            .arg("/q")
            .arg("/c")
            .env("TEMP", tmp_path)
            .env("TMP", tmp_path)
            .env(arg_name, args.as_str())
            .arg(format!("%{}%", arg_name))
            .status()
            .expect("failed to execute process")
    }
}

#[cfg(not(windows))]
fn try_run_with_new_temp(cwd: &str, executable: &str) -> process::ExitStatus {
    let tmp_dir = TempDir::new_in(&cwd).unwrap_or_else(|_| {
        error_exit!("Cannot create temporary directory");
    });
    let tmp_path = &tmp_dir.path();
    if is_absolute_path(executable) {
        process::Command::new(&executable)
            .env("TMPDIR", tmp_path)
            .args(env::args().skip(1))
            .status()
            .expect("failed to execute process")
    } else {
        process::Command::new("sh")
            .arg("-c")
            .env("TMPDIR", tmp_path)
            .arg(&executable)
            .args(env::args().skip(1))
            .status()
            .expect("failed to execute process")
    }
}

fn main() {
    let executable = get_pretend_executable();
    let cwd = get_base_dir();
    let max_trial = get_max_trial();

    let mut retry_times: u8 = 0;
    let mut last_exit_code: i32;

    loop {
        if retry_times > 0 {
            println!("Retry attempt: {}", retry_times);
        }
        let status = try_run_with_new_temp(&cwd, &executable);
        retry_times += 1;
        last_exit_code = status.code().unwrap_or(1);
        if status.success() || retry_times > max_trial {
            break;
        }
    }

    process::exit(last_exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_exists_relative_pass() {
        assert_eq!(path_exists("."), true);
    }

    #[test]
    fn test_dir_exists_relative_pass() {
        assert_eq!(dir_exists("."), true);
    }
    #[test]
    fn test_path_exists_absolute_pass() {
        let e = get_current_exe().ok();
        e.and_then(|p| {
            assert_eq!(path_exists(&p), true);
            Some(true)
        });
    }

    #[test]
    fn test_dir_exists_absolute_pass() {
        let d = get_current_dir().ok();
        d.and_then(|p| {
            assert_eq!(dir_exists(&p), true);
            Some(true)
        });
    }

    #[test]
    fn test_dir_exists_absolute_fail() {
        let e = get_current_exe().ok();
        e.and_then(|p| {
            assert_eq!(dir_exists(&p), false);
            Some(true)
        });
    }

    #[test]
    fn test_tempdir_create_state() {
        TempDir::new().ok().and_then(|d| {
            d.path().to_str().and_then(|p| {
                assert_eq!(dir_exists(p), true);
                Some(true)
            });
            Some(true)
        });
    }

    #[test]
    fn test_is_same_file_stem() {
        let file = PathBuf::from("randomtemp");
        let file_ext = PathBuf::from("randomtemp.exe");
        let no_file: Option<&PathBuf> = None;

        assert_eq!(is_same_file_stem(Some(&file), Some(&file_ext)), true);
        assert_eq!(is_same_file_stem(Some(&file), no_file), false);
        assert_eq!(is_same_file_stem(Some(&file_ext), no_file), false);
    }

    #[cfg(windows)]
    const ENV_COMMAND: &str = "set";

    #[cfg(not(windows))]
    const ENV_COMMAND: &str = "env";

    #[test]
    fn test_executable_with_env() {
        let original_env = if cfg!(windows) {
            process::Command::new("cmd")
                .arg("/q")
                .arg("/c")
                .arg(ENV_COMMAND)
                .output()
                .expect("failed to execute process")
        } else {
            process::Command::new("sh")
                .arg("-c")
                .arg(ENV_COMMAND)
                .args(env::args().skip(1))
                .output()
                .expect("failed to execute process")
        };
        let d = get_current_dir().ok();
        d.and_then(|p| {
            let new_env = process::Command::new("randomtemp")
                .env("RANDOMTEMP_EXECUTABLE", ENV_COMMAND)
                .env_remove("RANDOMTEMP_BASEDIR")
                .env_remove("RANDOMTEMP_MAXTRIAL")
                .current_dir(p)
                .output()
                .expect("failed to execute process");

            let original_output = original_env.stdout;
            let new_output = new_env.stdout;
            let original_error = original_env.stderr;
            let new_error = new_env.stderr;

            assert_ne!(original_output, new_output);
            assert_ne!(original_output.len(), 0);
            assert_ne!(new_output.len(), 0);
            assert_eq!(original_error.len(), 0);
            assert_eq!(new_error.len(), 0);
            assert_eq!(original_env.status.success(), true);
            assert_eq!(new_env.status.success(), true);
            Some(true)
        });
    }

    #[test]
    fn test_executable_with_invalid_max_retry_times() {
        let d = get_current_dir().ok();
        d.and_then(|p| {
            let new_env = process::Command::new("randomtemp")
                .env("RANDOMTEMP_EXECUTABLE", ENV_COMMAND)
                .env("RANDOMTEMP_MAXTRIAL", "-1")
                .env_remove("RANDOMTEMP_BASEDIR")
                .current_dir(p)
                .output()
                .expect("failed to execute process");

            let new_output = new_env.stdout;
            let new_error = new_env.stderr;

            assert_eq!(new_output.len(), 0);
            assert_ne!(new_error.len(), 0);
            assert_eq!(new_env.status.success(), false);
            Some(true)
        });
    }

    #[test]
    fn test_executable_with_invalid_base_dir() {
        let d = get_current_dir().ok();
        d.and_then(|p| {
            let new_env = process::Command::new("randomtemp")
                .env("RANDOMTEMP_EXECUTABLE", ENV_COMMAND)
                .env("RANDOMTEMP_BASEDIR", "randomtemp")
                .env_remove("RANDOMTEMP_MAXTRIAL")
                .current_dir(p)
                .output()
                .expect("failed to execute process");

            let new_output = new_env.stdout;
            let new_error = new_env.stderr;

            assert_eq!(new_output.len(), 0);
            assert_ne!(new_error.len(), 0);
            assert_eq!(new_env.status.success(), false);
            Some(true)
        });
    }

    #[test]
    fn test_executable_with_no_pretend_self_with_env() {
        let d = get_current_dir().ok();
        d.and_then(|p| {
            let new_env = process::Command::new("randomtemp")
                .env("RANDOMTEMP_EXECUTABLE", "randomtemp")
                .env_remove("RANDOMTEMP_BASEDIR")
                .env_remove("RANDOMTEMP_MAXTRIAL")
                .current_dir(p)
                .output()
                .expect("failed to execute process");

            let new_output = new_env.stdout;
            let new_error = new_env.stderr;

            assert_eq!(new_output.len(), 0);
            assert_ne!(new_error.len(), 0);
            assert_eq!(new_env.status.success(), false);
            Some(true)
        });
    }

    #[test]
    fn test_executable_with_no_pretend_self_with_name() {
        let d = get_current_dir().ok();
        d.and_then(|p| {
            let new_env = process::Command::new("randomtemp")
                .env_remove("RANDOMTEMP_EXECUTABLE")
                .env_remove("RANDOMTEMP_BASEDIR")
                .env_remove("RANDOMTEMP_MAXTRIAL")
                .current_dir(p)
                .output()
                .expect("failed to execute process");

            let new_output = new_env.stdout;
            let new_error = new_env.stderr;

            assert_eq!(new_output.len(), 0);
            assert_ne!(new_error.len(), 0);
            assert_eq!(new_env.status.success(), false);
            Some(true)
        });
    }

    #[cfg(windows)]
    #[test]
    fn test_executable_with_shell_quoting_windows() {
        let d = get_current_dir().ok();
        d.and_then(|p| {
            let new_env = process::Command::new("randomtemp")
                .env("RANDOMTEMP_EXECUTABLE", "dir")
                .env_remove("RANDOMTEMP_BASEDIR")
                .env_remove("RANDOMTEMP_MAXTRIAL")
                .arg("C:\\Program Files")
                .current_dir(p)
                .output()
                .expect("failed to execute process");

            let new_output = new_env.stdout;
            let new_error = new_env.stderr;

            assert_ne!(new_output.len(), 0);
            assert_eq!(new_error.len(), 0);
            assert_eq!(new_env.status.success(), true);
            Some(true)
        });
    }

    #[test]
    fn test_pretend_executable_with_absolute_path() -> Result<(), Box<dyn std::error::Error>> {
        let executable_name = if cfg!(windows) {
            "test_randomtemp_absolute.exe"
        } else {
            "test_randomtemp_absolute"
        };

        let tmp_dir_1 = TempDir::new()?;
        let mut tmp_path_1 = tmp_dir_1.path().to_owned();
        tmp_path_1.push(executable_name);

        let tmp_dir_2 = TempDir::new()?;
        let mut tmp_path_2 = tmp_dir_2.path().to_owned();
        tmp_path_2.push(executable_name);

        {
            fs::File::create(&tmp_path_1)?;
            fs::File::create(&tmp_path_2)?;
        }

        if let Some(path) = env::var_os("PATH") {
            let mut paths = env::split_paths(&path).collect::<Vec<_>>();
            paths.push(tmp_dir_1.path().to_path_buf());
            paths.push(tmp_dir_2.path().to_path_buf());
            let new_path = env::join_paths(paths)?;
            env::set_var("PATH", &new_path);
        }

        let actual_executable = tmp_path_2.to_str().unwrap();
        env::set_var("RANDOMTEMP_EXECUTABLE", actual_executable);
        let pred_executable = get_pretend_executable();
        assert_eq!(actual_executable, pred_executable);
        return Ok(());
    }
}
