use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use opener::open;

struct Config {
    s3_path: Option<String>,
    last_synced: Option<String>,
}

fn config_dir() -> PathBuf {
    env::home_dir().unwrap().join(".config").join("note")
}

fn config_file() -> PathBuf {
    config_dir().join("config")
}

fn notes_file() -> PathBuf {
    config_dir().join("notes.temp")
}

fn conflict_file() -> PathBuf {
    config_dir().join("notes.temp.remote")
}

fn read_config() -> Config {
    let mut cfg = Config { s3_path: None, last_synced: None };
    if let Ok(contents) = fs::read_to_string(config_file()) {
        for line in contents.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "s3_path" => cfg.s3_path = Some(value.to_string()),
                    "last_synced" => cfg.last_synced = Some(value.to_string()),
                    _ => {}
                }
            }
        }
    }
    cfg
}

fn write_config(cfg: &Config) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(config_dir())?;
    let mut contents = String::new();
    if let Some(s3_path) = &cfg.s3_path {
        contents.push_str(&format!("s3_path={}\n", s3_path));
    }
    if let Some(last_synced) = &cfg.last_synced {
        contents.push_str(&format!("last_synced={}\n", last_synced));
    }
    fs::write(config_file(), contents)?;
    Ok(())
}

fn split_bucket_key(s3_path: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let rest = s3_path.strip_prefix("s3://").ok_or("s3 path must start with s3://")?;
    let (bucket, key) = rest.split_once('/').ok_or("s3 path must include a key, e.g. s3://bucket/note/notes.temp")?;
    if key.is_empty() {
        return Err("s3 path must not point at the bucket root".into());
    }
    Ok((bucket.to_string(), key.to_string()))
}

fn head_object_last_modified(s3_path: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let (bucket, key) = split_bucket_key(s3_path)?;
    let output = Command::new("aws")
        .args(["s3api", "head-object", "--bucket", &bucket, "--key", &key, "--query", "LastModified", "--output", "text"])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let last_modified = String::from_utf8(output.stdout)?.trim().to_string();
    Ok(Some(last_modified))
}

fn s3_pull(s3_path: &str, local_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("aws")
        .args(["s3", "cp", s3_path, local_path.to_str().unwrap()])
        .status()?;
    if !status.success() {
        return Err("aws s3 cp (pull) failed".into());
    }
    Ok(())
}

fn s3_push(local_path: &PathBuf, s3_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("aws")
        .args(["s3", "cp", local_path.to_str().unwrap(), s3_path])
        .status()?;
    if !status.success() {
        return Err("aws s3 cp (push) failed".into());
    }
    Ok(())
}

fn require_s3_path(cfg: &Config) -> Result<String, Box<dyn std::error::Error>> {
    cfg.s3_path.clone().ok_or_else(|| "No S3 path configured. Run: note config s3://bucket/prefix/notes.temp".into())
}

fn sync_if_stale(cfg: &Config, s3_path: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    fs::create_dir_all(config_dir())?;
    let remote_last_modified = head_object_last_modified(s3_path)?;
    match &remote_last_modified {
        None => {}
        Some(remote) => {
            if cfg.last_synced.as_deref() != Some(remote.as_str()) {
                s3_pull(s3_path, &notes_file())?;
            }
        }
    }
    Ok(remote_last_modified)
}

fn print_conflict_diff(local_path: &PathBuf, remote_path: &PathBuf) {
    if let Ok(output) = Command::new("diff").args([local_path.to_str().unwrap(), remote_path.to_str().unwrap()]).output() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        return Err("Did not specify a note!".into());
    }

    if args.len() == 3 && args[1] == "config" {
        let mut cfg = read_config();
        cfg.s3_path = Some(args[2].clone());
        cfg.last_synced = None;
        write_config(&cfg)?;
        println!("Configured s3 path: {}", args[2]);
        return Ok(());
    }

    let mut cfg = read_config();
    let s3_path = require_s3_path(&cfg)?;

    if args.len() == 2 && args[1] == "open" {
        sync_if_stale(&cfg, &s3_path)?;
        open(&notes_file())?;
        return Ok(());
    }

    if args.len() == 3 && args[1] == "push" && args[2] == "--force" {
        s3_push(&notes_file(), &s3_path)?;
        let new_last_modified = head_object_last_modified(&s3_path)?;
        cfg.last_synced = new_last_modified;
        write_config(&cfg)?;
        println!("Force pushed local notes to {}", s3_path);
        return Ok(());
    }

    let pre_append_last_modified = sync_if_stale(&cfg, &s3_path)?;

    let arg_line = args[1..].join(" ");
    let mut f = fs::OpenOptions::new().append(true).create(true).open(notes_file())?;
    writeln!(&mut f, "{}", arg_line)?;
    drop(f);

    let pre_push_last_modified = head_object_last_modified(&s3_path)?;
    if pre_push_last_modified != pre_append_last_modified {
        s3_pull(&s3_path, &conflict_file())?;
        println!("Remote changed while writing this note. Not pushed.");
        println!("Your local copy (with the new note) is at {:?}", notes_file());
        println!("The remote copy is at {:?}", conflict_file());
        print_conflict_diff(&notes_file(), &conflict_file());
        println!("Resolve manually, then run: note push --force");
        return Ok(());
    }

    s3_push(&notes_file(), &s3_path)?;
    let new_last_modified = head_object_last_modified(&s3_path)?;
    cfg.last_synced = new_last_modified;
    write_config(&cfg)?;

    Ok(())
}
