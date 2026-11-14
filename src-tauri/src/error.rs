use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("invalid settings file: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("could not serialize settings: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("{0}")]
    Msg(String),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub fn msg(s: impl Into<String>) -> Self {
        Error::Msg(s.into())
    }
}

// Commands return errors to the frontend as plain strings.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// The last non-empty line a program printed (usually its error), trimmed; what LampRabit
/// shows when a tool it ran fails.
pub fn last_line(output: &[u8]) -> Option<String> {
    String::from_utf8_lossy(output).lines().rev().map(str::trim).find(|l| !l.is_empty()).map(str::to_string)
}
