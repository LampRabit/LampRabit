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

/// What a tool's error output means when it's a known problem with the computer rather
/// than the project, and what to do about it.
pub fn explain(output: &str) -> Option<&'static str> {
    const CERTIFICATE: [&str; 5] = ["UNABLE_TO_VERIFY_LEAF_SIGNATURE", "SELF_SIGNED_CERT_IN_CHAIN", "UNABLE_TO_GET_ISSUER_CERT_LOCALLY", "CERTIFICATE_VERIFY_FAILED", "certificate verify failed"];
    CERTIFICATE.iter().any(|c| output.contains(c)).then_some(
        "Something on this network re-signs HTTPS connections (a company proxy, VPN or antivirus), and the tool doesn't trust its certificate the way your browser does. LampRabit asks Node to use your system's certificates, which needs Node.js 22.19 or 24.6 or newer: install a newer Node on Services → Node.js and try again. Otherwise ask your IT team for the proxy's certificate (npm: npm config set cafile <file>; Python: pip config set global.cert <file>).",
    )
}

/// The last non-empty line a program printed (usually its error), trimmed; what LampRabit
/// shows when a tool it ran fails.
pub fn last_line(output: &[u8]) -> Option<String> {
    String::from_utf8_lossy(output).lines().rev().map(str::trim).find(|l| !l.is_empty()).map(str::to_string)
}

#[cfg(test)]
mod tests {
    #[test]
    fn explains_certificate_failures_only() {
        assert!(super::explain("npm error code UNABLE_TO_VERIFY_LEAF_SIGNATURE").is_some_and(|h| h.contains("Node.js 22.19")));
        assert!(super::explain("[SSL: CERTIFICATE_VERIFY_FAILED] certificate verify failed").is_some());
        assert!(super::explain("npm error 404 Not Found - GET https://registry.npmjs.org/nope").is_none());
    }
}
