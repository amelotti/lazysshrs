use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SshHost {
    pub name: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub other_options: HashMap<String, String>,
    pub is_separator: bool,
    pub source_dir: Option<String>,
}

pub struct SshConfig {
    pub hosts: Vec<SshHost>,
}

impl SshConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let home_dir = home::home_dir().ok_or("Could not find home directory")?;
        let workdir = home_dir.join(".ssh");
        Self::load_from_workdir(&workdir)
    }

    pub fn load_from_workdir(workdir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = workdir.join("config");
        Self::load_file(&config_path)
    }

    fn load_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let base_dir = path.parent().unwrap_or(Path::new("/"));
        let source_dir = path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).map(|s| s.to_string());
        Self::parse(&content, base_dir, source_dir)
    }



    fn parse(content: &str, base_dir: &Path, source_dir: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut hosts = Vec::new();
        let mut current_host: Option<SshHost> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0].to_lowercase();
            let value = parts[1].trim();

            match key.as_str() {
                "include" => {
                    if let Some(host) = current_host.take() {
                        hosts.push(host);
                    }
                    let include_path = Self::resolve_include_path(value, base_dir)?;
                    if include_path.exists() {
                        let dir_name = include_path.parent()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        
                        hosts.push(SshHost {
                            name: format!("── {} ──", dir_name),
                            hostname: None,
                            user: None,
                            port: None,
                            identity_file: None,
                            other_options: HashMap::new(),
                            is_separator: true,
                            source_dir: Some(dir_name.clone()),
                        });
                        
                        let included_config = Self::load_file(&include_path)?;
                        hosts.extend(included_config.hosts);
                    }
                }
                "host" => {
                    if let Some(host) = current_host.take() {
                        hosts.push(host);
                    }
                    current_host = Some(SshHost {
                        name: value.to_string(),
                        hostname: None,
                        user: None,
                        port: None,
                        identity_file: None,
                        other_options: HashMap::new(),
                        is_separator: false,
                        source_dir: source_dir.clone(),
                    });
                }
                "hostname" => {
                    if let Some(ref mut host) = current_host {
                        host.hostname = Some(value.to_string());
                    }
                }
                "user" => {
                    if let Some(ref mut host) = current_host {
                        host.user = Some(value.to_string());
                    }
                }
                "port" => {
                    if let Some(ref mut host) = current_host {
                        host.port = value.parse().ok();
                    }
                }
                "identityfile" => {
                    if let Some(ref mut host) = current_host {
                        host.identity_file = Some(value.to_string());
                    }
                }
                _ => {
                    if let Some(ref mut host) = current_host {
                        host.other_options.insert(key, value.to_string());
                    }
                }
            }
        }

        if let Some(host) = current_host {
            hosts.push(host);
        }

        Ok(Self { hosts })
    }

    fn resolve_include_path(include_value: &str, base_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let path = if include_value.starts_with('~') {
            let home_dir = home::home_dir().ok_or("Could not find home directory")?;
            home_dir.join(&include_value[2..])
        } else if include_value.starts_with('/') {
            PathBuf::from(include_value)
        } else {
            base_dir.join(include_value)
        };
        Ok(path)
    }
}