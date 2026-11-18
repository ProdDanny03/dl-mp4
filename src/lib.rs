use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs, thread};

pub fn strip_wrapping_quotes(value: &str) -> &str {
    let mut value = value;
    while let Some(inner) = strip_one_pair(value) {
        value = inner;
    }
    value
}

fn strip_one_pair(value: &str) -> Option<&str> {
    let bytes = value.as_bytes();
    let (&first, rest) = bytes.split_first()?;
    let (&last, _) = rest.split_last()?;
    if matches!(first, b'"' | b'\'') && first == last {
        Some(&value[1..value.len() - 1])
    } else {
        None
    }
}

pub fn downloads_dir() -> String {
    env::var("USERPROFILE")
        .ok()
        .map(|profile| PathBuf::from(profile).join("Downloads"))
        .and_then(|path| path.into_os_string().into_string().ok())
        .unwrap_or_default()
}

pub fn folder_config_file() -> Option<PathBuf> {
    let appdata = env::var_os("APPDATA")?;
    Some(PathBuf::from(appdata).join("dl-mp4").join("folder.txt"))
}

pub fn folder_is_usable(path: &str) -> bool {
    let path = strip_wrapping_quotes(path.trim());
    !path.is_empty() && Path::new(path).is_dir()
}

pub fn save_folder_path(path: &str) -> io::Result<()> {
    let Some(config) = folder_config_file() else {
        return Err(io::Error::other("APPDATA is not set"));
    };
    if let Some(parent) = config.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(config, path)
}

pub fn urls_from_input(input: &str) -> Result<Vec<String>, String> {
    let path = Path::new(input);
    let is_txt_file = path.is_file()
        && path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"));
    if !is_txt_file {
        return Ok(vec![input.to_string()]);
    }

    let text = fs::read_to_string(path)
        .map_err(|err| format!("Error: could not read URL file ({err})."))?;
    let urls: Vec<String> = text
        .lines()
        .map(|line| line.trim_start_matches('\u{feff}'))
        .map(|line| strip_wrapping_quotes(line.trim()).to_string())
        .filter(|line| !line.is_empty())
        .collect();
    if urls.is_empty() {
        return Err("Error: the text file does not contain any URLs.".to_string());
    }
    Ok(urls)
}

fn ytdlp_args(path: &str) -> Vec<String> {
    vec![
        "--newline".to_string(),
        "--downloader".to_string(),
        "aria2c".to_string(),
        "--extractor-args".to_string(),
        "generic:impersonate".to_string(),
        "-P".to_string(),
        path.to_string(),
        "-t".to_string(),
        "mp4".to_string(),
        "-a".to_string(),
        "-".to_string(),
    ]
}

pub fn run_ytdlp(path: &str, urls: &[String], emit: impl Fn(String) + Clone + Send + 'static) {
    let mut command = Command::new("yt-dlp");
    command
        .args(ytdlp_args(path))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: yt-dlp is a console program, so without this Windows opens a terminal.
        command.creation_flags(0x08000000);
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            emit(format!("Failed to start yt-dlp: {err}"));
            return;
        }
    };

    let emit_stdout = emit.clone();
    let stdout = child.stdout.take();
    let stdout_thread = thread::spawn(move || forward_lines(stdout, emit_stdout));

    let emit_stderr = emit.clone();
    let stderr = child.stderr.take();
    let stderr_thread = thread::spawn(move || forward_lines(stderr, emit_stderr));

    if let Some(mut stdin) = child.stdin.take()
        && let Err(err) = write_urls(&mut stdin, urls)
    {
        emit(format!("Failed to write URLs to yt-dlp: {err}"));
    }

    let status = child.wait();
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    let line = match status {
        Ok(status) => format!("yt-dlp exited with {status}"),
        Err(err) => format!("Failed to wait for yt-dlp: {err}"),
    };
    emit(line);
}

fn write_urls(stdin: &mut impl Write, urls: &[String]) -> io::Result<()> {
    for url in urls {
        stdin.write_all(url.as_bytes())?;
        stdin.write_all(b"\n")?;
    }
    Ok(())
}

fn forward_lines(pipe: Option<impl Read + Send>, emit: impl Fn(String)) {
    let Some(pipe) = pipe else {
        return;
    };

    let mut reader = BufReader::new(pipe);
    let mut chunk = [0u8; 4096];
    let mut pending = Vec::new();

    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => consume_output(&mut pending, &chunk[..n], &emit),
            Err(err) => {
                emit(format!("Failed to read process output: {err}"));
                return;
            }
        }
    }

    emit_line(&pending, &emit);
}

fn consume_output(pending: &mut Vec<u8>, chunk: &[u8], emit: &impl Fn(String)) {
    let mut start = 0;
    for (index, &byte) in chunk.iter().enumerate() {
        if byte == b'\n' || byte == b'\r' {
            pending.extend_from_slice(&chunk[start..index]);
            emit_line(pending, emit);
            pending.clear();
            start = index + 1;
        }
    }
    pending.extend_from_slice(&chunk[start..]);
}

fn emit_line(bytes: &[u8], emit: &impl Fn(String)) {
    let line = String::from_utf8_lossy(bytes).trim().to_string();
    if !line.is_empty() {
        emit(line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_matching_wrapping_quotes() {
        assert_eq!(
            strip_wrapping_quotes("\"https://example.com\""),
            "https://example.com"
        );
        assert_eq!(
            strip_wrapping_quotes("'https://example.com'"),
            "https://example.com"
        );
        assert_eq!(
            strip_wrapping_quotes("\"'https://example.com'\""),
            "https://example.com"
        );
    }

    #[test]
    fn keeps_unbalanced_quotes() {
        assert_eq!(
            strip_wrapping_quotes("\"https://example.com"),
            "\"https://example.com"
        );
        assert_eq!(strip_wrapping_quotes("it's"), "it's");
        assert_eq!(strip_wrapping_quotes("\"C:\\dir'"), "\"C:\\dir'");
    }

    #[test]
    fn ytdlp_args_read_urls_from_stdin() {
        let args = ytdlp_args(r"C:\Users\me\Downloads");
        assert_eq!(
            args,
            [
                "--newline",
                "--downloader",
                "aria2c",
                "--extractor-args",
                "generic:impersonate",
                "-P",
                r"C:\Users\me\Downloads",
                "-t",
                "mp4",
                "-a",
                "-",
            ]
        );
    }
}
