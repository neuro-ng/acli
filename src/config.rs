use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EscalationSchedule {
    pub name: String,
    pub schedule_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Defaults {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bb_project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalation_schedules: Option<Vec<EscalationSchedule>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Profile {
    pub name: String,
    pub atlassian_url: String,
    pub email: String,
    pub api_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<Defaults>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,
    pub profiles: HashMap<String, Profile>,
}

impl Config {
    pub fn config_dir() -> Result<PathBuf, String> {
        let home = std::env::var("HOME")
            .map_err(|_| "Could not find HOME environment variable. Please set it.".to_string())?;
        Ok(PathBuf::from(home).join(".config").join("acli"))
    }

    pub fn load() -> Result<Self, String> {
        let dir = Self::config_dir()?;
        let path = dir.join("config.json");

        if !path.exists() {
            return Ok(Config {
                default_profile: None,
                profiles: HashMap::new(),
            });
        }

        let data = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read config file {}: {}", path.display(), e))?;

        let cfg: Config = serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse config JSON: {}", e))?;

        Ok(cfg)
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;

        let path = dir.join("config.json");
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config to JSON: {}", e))?;

        fs::write(&path, data)
            .map_err(|e| format!("Failed to write config to file {}: {}", path.display(), e))?;

        Ok(())
    }

    pub fn get_profile(&self, name: Option<&str>) -> Result<Profile, String> {
        if let Some(n) = name {
            if let Some(p) = self.profiles.get(n) {
                return Ok(p.clone());
            } else {
                return Err(format!("Profile \"{}\" not found", n));
            }
        }

        if let Some(ref default_name) = self.default_profile {
            if let Some(p) = self.profiles.get(default_name) {
                return Ok(p.clone());
            }
        }

        if self.profiles.len() == 1 {
            if let Some((_, p)) = self.profiles.iter().next() {
                return Ok(p.clone());
            }
        }

        if self.profiles.is_empty() {
            return Err(
                "No profiles configured, run 'acli config setup' to create one".to_string(),
            );
        }

        Err("Multiple profiles configured, specify one with -p or set a default with 'acli config set-default <name>'".to_string())
    }
}

pub fn run_setup(profile_name: &str) -> Result<(), String> {
    let mut cfg = Config::load()?;
    let existing = cfg.profiles.get(profile_name);

    if existing.is_some() {
        println!(
            "Updating profile \"{}\" (press Enter to keep current value)\n",
            profile_name
        );
    } else {
        println!("Creating profile \"{}\"\n", profile_name);
    }

    let existing_url = existing.map(|p| p.atlassian_url.as_str()).unwrap_or("");
    let existing_email = existing.map(|p| p.email.as_str()).unwrap_or("");
    let existing_token = existing.map(|p| p.api_token.as_str()).unwrap_or("");

    let atlassian_url = prompt_with_default(
        "Atlassian URL",
        existing_url,
        "https://your-instance.atlassian.net",
    )?;

    let email = prompt_with_default("Email", existing_email, "")?;

    let masked_token = mask_token(existing_token);
    let api_token_input = prompt_with_default("API Token", &masked_token, "")?;

    let api_token = if api_token_input == masked_token && !existing_token.is_empty() {
        existing_token.to_string()
    } else {
        api_token_input
    };

    let profile = Profile {
        name: profile_name.to_string(),
        atlassian_url: atlassian_url.trim_end_matches('/').to_string(),
        email,
        api_token,
        defaults: existing.and_then(|p| p.defaults.clone()),
    };

    let is_first = cfg.profiles.is_empty();
    cfg.profiles.insert(profile_name.to_string(), profile);

    if is_first || cfg.default_profile.is_none() {
        cfg.default_profile = Some(profile_name.to_string());
    }

    cfg.save()?;

    println!(
        "\nProfile \"{}\" saved to ~/.config/acli/config.json",
        profile_name
    );
    if is_first || cfg.default_profile.as_deref() == Some(profile_name) {
        println!("Profile \"{}\" is the default profile", profile_name);
    }

    Ok(())
}

pub fn run_list() -> Result<(), String> {
    let cfg = Config::load()?;
    if cfg.profiles.is_empty() {
        println!("No profiles configured. Run 'acli config setup' to create one.");
        return Ok(());
    }

    println!(
        "{:<20} {:<10} {:<40} {:<30}",
        "PROFILE", "DEFAULT", "URL", "EMAIL"
    );
    for (name, profile) in &cfg.profiles {
        let is_default = if cfg.default_profile.as_deref() == Some(name) {
            "*"
        } else {
            ""
        };
        println!(
            "{:<20} {:<10} {:<40} {:<30}",
            name, is_default, profile.atlassian_url, profile.email
        );
    }
    Ok(())
}

pub fn run_show(profile_name: Option<&str>) -> Result<(), String> {
    let cfg = Config::load()?;
    let p = cfg.get_profile(profile_name)?;

    println!("Profile: {}", p.name);
    println!("  Atlassian URL:    {}", p.atlassian_url);
    println!("  Email:            {}", p.email);
    println!("  API Token:        {}", mask_token(&p.api_token));
    if let Some(ref defs) = p.defaults {
        println!("  Defaults:");
        if let Some(ref proj) = defs.project {
            println!("    Project:        {}", proj);
        }
        if let Some(ref ws) = defs.workspace {
            println!("    Workspace:      {}", ws);
        }
        if let Some(ref bbp) = defs.bb_project {
            println!("    BB Project:     {}", bbp);
        }
    }
    Ok(())
}

pub fn run_delete(profile_name: &str) -> Result<(), String> {
    let mut cfg = Config::load()?;
    if cfg.profiles.remove(profile_name).is_none() {
        return Err(format!("Profile \"{}\" not found", profile_name));
    }

    if cfg.default_profile.as_deref() == Some(profile_name) {
        cfg.default_profile = None;
        // Fall back to first remaining profile as default
        if let Some(first_name) = cfg.profiles.keys().next() {
            cfg.default_profile = Some(first_name.clone());
        }
    }

    cfg.save()?;
    println!("Profile \"{}\" deleted", profile_name);
    Ok(())
}

pub fn run_set_default(profile_name: &str) -> Result<(), String> {
    let mut cfg = Config::load()?;
    if !cfg.profiles.contains_key(profile_name) {
        return Err(format!("Profile \"{}\" not found", profile_name));
    }
    cfg.default_profile = Some(profile_name.to_string());
    cfg.save()?;
    println!("Default profile set to \"{}\"", profile_name);
    Ok(())
}

fn prompt_with_default(label: &str, current: &str, placeholder: &str) -> Result<String, String> {
    if !current.is_empty() {
        print!("  {} [{}]: ", label, current);
    } else if !placeholder.is_empty() {
        print!("  {} ({}): ", label, placeholder);
    } else {
        print!("  {}: ", label);
    }
    io::stdout()
        .flush()
        .map_err(|e| format!("Failed to flush stdout: {}", e))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed to read stdin: {}", e))?;

    let trimmed = input.trim();
    if trimmed.is_empty() {
        if !current.is_empty() {
            Ok(current.to_string())
        } else {
            Ok(placeholder.to_string())
        }
    } else {
        Ok(trimmed.to_string())
    }
}

fn mask_token(token: &str) -> String {
    if token.is_empty() {
        return String::new();
    }
    if token.len() <= 8 {
        return "****".to_string();
    }
    format!("{}****{}", &token[..4], &token[token.len() - 4..])
}
