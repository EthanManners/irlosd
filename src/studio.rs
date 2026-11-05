//! IrlosStudio's config and process. The config is rebuilt from the templates
//! before every launch, so every launch starts from the same known state.

use crate::config::Config;
use std::fs;
use std::io;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;
use std::process::{Child, Command, Stdio};

pub fn write_config(cfg: &Config) -> io::Result<()> {
    let config_home = cfg.state_dir.join("config");
    let out = config_home.join("obs-studio");
    // Keep logs/, replace everything else.
    for stale in ["basic", "global.ini", "user.ini"] {
        let path = out.join(stale);
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else if path.exists() {
            fs::remove_file(&path)?;
        }
    }
    render_dir(&cfg.template_dir.join("obs-studio"), &out, &cfg.vars)
}

fn render_dir(from: &Path, to: &Path, vars: &[(&str, String)]) -> io::Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            render_dir(&entry.path(), &dest, vars)?;
            continue;
        }
        let json = dest.extension().is_some_and(|e| e == "json");
        let mut text = fs::read_to_string(entry.path())?;
        for (name, value) in vars {
            let value = if json {
                json_escape(value)
            } else {
                value.clone()
            };
            text = text.replace(&format!("{{{{{name}}}}}"), &value);
        }
        // Files may hold the stream key.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&dest)?;
        io::Write::write_all(&mut file, text.as_bytes())?;
    }
    Ok(())
}

/// The inside of a JSON string literal.
fn json_escape(s: &str) -> String {
    let quoted = serde_json::to_string(s).expect("strings always serialize");
    quoted[1..quoted.len() - 1].to_string()
}

pub fn spawn(cfg: &Config) -> io::Result<Child> {
    Command::new(&cfg.obs)
        .arg("--headless")
        .current_dir(cfg.obs.parent().unwrap_or(Path::new("/")))
        .env("XDG_CONFIG_HOME", cfg.state_dir.join("config"))
        .env("IRLOS_ICP_SOCKET", cfg.studio_socket())
        .env_remove("STREAM_KEY")
        .stdin(Stdio::null())
        .spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_templates() {
        let root = std::env::temp_dir().join(format!("irlosd-test-{}", std::process::id()));
        let (from, to) = (root.join("t"), root.join("o"));
        fs::create_dir_all(from.join("sub")).unwrap();
        fs::write(from.join("a.ini"), "Key={{WIDTH}}\n").unwrap();
        fs::write(from.join("sub/b.json"), r#"{"key":"{{STREAM_KEY}}"}"#).unwrap();
        let vars = [
            ("WIDTH", "1920".to_string()),
            ("STREAM_KEY", "k\"\\".to_string()),
        ];
        render_dir(&from, &to, &vars).unwrap();
        assert_eq!(fs::read_to_string(to.join("a.ini")).unwrap(), "Key=1920\n");
        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(to.join("sub/b.json")).unwrap()).unwrap();
        assert_eq!(json["key"], "k\"\\");
        fs::remove_dir_all(root).unwrap();
    }
}
