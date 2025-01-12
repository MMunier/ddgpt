use std::{
    error::Error,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

macro_rules! templ_fetch_userdir {
    ($fn_name:ident, $XDG_VAR_NAME:literal, $DEFAULT_PATH:literal) => {
        pub fn $fn_name() -> PathBuf {
            user_dir($XDG_VAR_NAME, $DEFAULT_PATH)
        }
    };
}

fn user_dir(xdg_variable: &'static str, default_user_dir: &'static str) -> PathBuf {
    let path = std::env::var(xdg_variable)
        .map(|dir| format!("{}/{}", dir, env!("CARGO_PKG_NAME")))
        .or_else(|_| {
            std::env::var("HOME").map(|home_dir| {
                format!(
                    "{}/{}/{}",
                    home_dir,
                    default_user_dir,
                    env!("CARGO_PKG_NAME")
                )
            })
        });

    if let Ok(path) = path {
        return PathBuf::from(path);
    }

    panic!("User environment did not yield sufficient info to determine config-dir");
}

templ_fetch_userdir!(user_cache_dir, "XDG_CACHE_HOME", "/.cache/");
templ_fetch_userdir!(user_config_dir, "XDG_CONFIG_HOME", "/.config/");
templ_fetch_userdir!(user_data_dir, "XDG_DATA_HOME", "/.local/share/");
templ_fetch_userdir!(user_state_dir, "XDG_STATE_HOME", "/.local/state/");

// templ_fetch_userdir!(user_cache_dir);
// templ_fetch_userdir!(user_cache_dir);
// templ_fetch_userdir!(user_cache_dir);
// templ_fetch_userdir!(user_cache_dir);

pub enum ConfigFileType {
    // #[cfg( = "JSON")]
    JSON,
    // #[cfg( = "TOML")]
    TOML,
}

pub type ConfigResult<T> = Result<T, ConfigError>;

#[derive(Debug)]
pub enum ConfigFormatError {
    JSON(serde_json::Error),
    TOML(Box<dyn Error>),
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Format(ConfigFormatError),
}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        ConfigError::Io(value)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(value: serde_json::Error) -> Self {
        ConfigError::Format(ConfigFormatError::JSON(value))
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(value: toml::ser::Error) -> Self {
        ConfigError::Format(ConfigFormatError::TOML(Box::new(value)))
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(value: toml::de::Error) -> Self {
        ConfigError::Format(ConfigFormatError::TOML(Box::new(value)))
    }
}

fn write_config_file<T: ConfigLoadable>(
    config_path: &std::path::Path,
    config: &T,
) -> ConfigResult<()> {
    std::fs::create_dir_all(
        config_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("")),
    )?;

    match T::FILETYPE {
        // #[cfg(predicate)]
        ConfigFileType::JSON => {
            let writer = BufWriter::new(std::fs::File::create(config_path)?);
            serde_json::to_writer_pretty(writer, &config)?
        }

        // #[cfg(predicate)]
        ConfigFileType::TOML => {
            std::fs::write(config_path, toml::to_string_pretty(&config)?)?;
        }
    }

    Ok(())
}

pub trait ConfigLoadable: Default + serde::Serialize + serde::de::DeserializeOwned {
    const FILENAME: &'static str;
    const FILETYPE: ConfigFileType;

    fn load() -> ConfigResult<Self> {
        let path = user_config_dir();
        let config_path = std::path::Path::new(&path).with_file_name(Self::FILENAME);

        let reader = BufReader::new(match std::fs::File::open(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let default_config = Self::default();
                write_config_file(&config_path, &default_config)?;
                return Ok(default_config);
            }
            a => a,
        }?);

        Ok(serde_json::from_reader(reader)?)
    }

    fn save(&self) -> Result<(), ConfigError> {
        let path = user_config_dir();
        let config_path = std::path::Path::new(&path).with_file_name(Self::FILENAME);
        write_config_file(&config_path, self)
    }
}
