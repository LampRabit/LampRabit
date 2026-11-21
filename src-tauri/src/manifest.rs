//! Service manifests: every service LampRabit runs is described by a YAML file: where its
//! downloads come from, its ports, the files it needs before it starts, how it's launched,
//! stopped and checked, and how apps connect to it. Built-in ones are in
//! `src-tauri/services/` (embedded at build time); community ones will come from the
//! LampRabit/Services registry. The format is documented in docs/services.md.
//!
//! What can't be described declaratively (vhosts and PHP pools for the web servers, data
//! folder set-up for the databases) stays in Rust behind named `builtin` hooks, which only
//! built-in manifests may use.

use std::{
    collections::BTreeMap,
    fmt,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
};

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, Visitor},
};

use crate::{
    config::Settings,
    error::{Error, Result},
    layout::Layout,
    runtimes::Archive,
};

/// The manifest format's version. Additive changes keep it; a manifest that needs a newer
/// app says so in `requires.lamprabit`.
pub const SCHEMA: u32 = 1;

/// The built-in services, in the order the supervisor lists them.
const BUILT_IN: &[&str] = &[
    include_str!("../services/apache.yaml"),
    include_str!("../services/nginx.yaml"),
    include_str!("../services/caddy.yaml"),
    include_str!("../services/frankenphp.yaml"),
    include_str!("../services/tomcat.yaml"),
    include_str!("../services/mariadb.yaml"),
    include_str!("../services/mysql.yaml"),
    include_str!("../services/postgres.yaml"),
    include_str!("../services/mongodb.yaml"),
    include_str!("../services/clickhouse.yaml"),
    include_str!("../services/neo4j.yaml"),
    include_str!("../services/meilisearch.yaml"),
    include_str!("../services/typesense.yaml"),
    include_str!("../services/rustfs.yaml"),
    include_str!("../services/minio.yaml"),
    include_str!("../services/elasticsearch.yaml"),
    include_str!("../services/opensearch.yaml"),
    include_str!("../services/rabbitmq.yaml"),
    include_str!("../services/ollama.yaml"),
    include_str!("../services/n8n.yaml"),
    include_str!("../services/mailpit.yaml"),
    include_str!("../services/redis.yaml"),
    include_str!("../services/memcached.yaml"),
    include_str!("../services/pgadmin.yaml"),
];

/// The groups of the Services page a manifest can name.
pub const CATEGORIES: &[&str] = &["web", "languages", "packages", "databases", "search", "storage", "queues", "ai", "cache", "tools", "sharing"];

// revision, description, homepage, license, maintainers, icon and the ports' labels are
// for the registry and the marketplace page (docs/services.md, the next phases).
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    /// Lowercase letters, digits and dashes: the folder name under bin/, etc/, data/.
    pub id: String,
    pub name: String,
    /// The manifest's own version, raised with every change; unrelated to the software's.
    #[serde(default = "one")]
    pub revision: u32,
    #[serde(default)]
    pub requires: Requires,
    pub category: String,
    /// A few words under the name on the Services page.
    pub blurb: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Where the downloads come from and anything worth knowing, on the Services page.
    #[serde(default)]
    pub origin: Option<Text>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub maintainers: Vec<String>,
    #[serde(default)]
    pub icon: Option<String>,
    /// Why it can't run on some platforms; a value that matches this platform makes it
    /// unavailable here.
    #[serde(default)]
    pub unavailable: Option<Text>,
    pub source: Sources,
    /// The file, inside the install, whose presence means the install is complete.
    pub binary: Text,
    #[serde(default)]
    pub needs: Vec<Need>,
    #[serde(default)]
    pub system_needs: Vec<SystemNeed>,
    #[serde(default)]
    pub suggests: Vec<String>,
    /// Named ports; the first is the main one (shown on its card, checked by default).
    #[serde(default)]
    pub ports: Ordered<PortDef>,
    /// Where `{{data}}` points.
    #[serde(default)]
    pub data: DataMode,
    /// Folders created before every start.
    #[serde(default)]
    pub dirs: Vec<Arg>,
    /// Files written (or copied) before every start, in order.
    #[serde(default)]
    pub files: Vec<FileOp>,
    /// Options the user can change on the service's card, used as `{{setting.<name>}}` and
    /// in `if: setting.<name>`.
    #[serde(default)]
    pub settings: Ordered<SettingDef>,
    #[serde(default)]
    pub run: Option<Run>,
    #[serde(default)]
    pub stop: Option<Stop>,
    pub health: Health,
    #[serde(default)]
    pub behaviour: Behaviour,
    #[serde(default)]
    pub connection: Option<Connection>,
    /// A web interface, served at https://<domain>.test through the web server.
    #[serde(default)]
    pub ui: Option<Ui>,
    /// The service's own log in logs/; `<id>-output.log` (its output) when it has none.
    #[serde(default)]
    pub log: Option<String>,
    /// What "Open config" opens.
    #[serde(default)]
    pub config: Option<Text>,
    /// Parts handled by LampRabit's own code (built-in manifests only).
    #[serde(default)]
    pub builtin: Vec<Hook>,
    /// Values that differ by platform or Linux distribution, as `{var.<name>}` in a
    /// source (MongoDB's build per distribution). The first case that matches wins.
    #[serde(default)]
    pub vars: Ordered<Vec<crate::sources::VarCase>>,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requires {
    /// The oldest LampRabit that understands this manifest, as "0.8.0".
    #[serde(default)]
    pub lamprabit: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Hook {
    /// Ports come from the web server settings.
    Ports,
    /// Config and first-run set-up before start (vhosts, data folders).
    Prepare,
    /// The command line is assembled in Rust.
    Launch,
    /// Something not started by LampRabit may already be running it (the Ollama app).
    External,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DataMode {
    /// data/<id>.
    #[default]
    Shared,
    /// data/<id>-<major>: for software whose files only work with the major version that
    /// wrote them.
    PerMajor,
    /// The storage folder from Settings (object storage, which can live anywhere).
    Storage,
}

/// Text that may depend on the platform: a plain string, one `{value, on}`, or a list of
/// them where the first that matches wins.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Text {
    Plain(String),
    One(Cond),
    Choice(Vec<Cond>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cond {
    pub value: String,
    /// Only when this setting is on (`setting.<name>`, or `!setting.<name>` for off).
    #[serde(default, rename = "if")]
    pub when: Option<String>,
    /// Platforms it applies to (`windows`, `linux`, `macos`, `unix`, `x64`, `arm64`, or
    /// `<os>-<arch>`); empty means all.
    #[serde(default)]
    pub on: Vec<String>,
}

impl Text {
    /// The value for this platform, if any applies.
    pub fn pick(&self) -> Option<&str> {
        match self {
            Text::Plain(s) => Some(s),
            Text::One(c) => matches(&c.on).then_some(c.value.as_str()),
            Text::Choice(list) => list.iter().find(|c| matches(&c.on)).map(|c| c.value.as_str()),
        }
    }

    fn all(&self) -> Vec<&str> {
        match self {
            Text::Plain(s) => vec![s],
            Text::One(c) => vec![&c.value],
            Text::Choice(list) => list.iter().map(|c| c.value.as_str()).collect(),
        }
    }

    fn platforms(&self) -> Vec<&str> {
        match self {
            Text::Plain(_) => vec![],
            Text::One(c) => c.on.iter().map(String::as_str).collect(),
            Text::Choice(list) => list.iter().flat_map(|c| c.on.iter().map(String::as_str)).collect(),
        }
    }
}

/// An option the user can change, shown on the service's card.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingDef {
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub kind: SettingKind,
    pub default: SettingValue,
    /// The choices (`choice`).
    #[serde(default)]
    pub options: Vec<String>,
    /// Limits (`number`).
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub max: Option<i64>,
    /// Shown after a number ("MB").
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingKind {
    /// One line of text.
    Text,
    /// A whole number, within `min` and `max`.
    Number,
    /// On or off: `true` or `false`.
    Toggle,
    /// One of `options`.
    Choice,
}

/// A default as written in YAML (`128`, `true`, `warning`).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum SettingValue {
    Bool(bool),
    Int(i64),
    Text(String),
}

impl SettingValue {
    pub fn text(&self) -> String {
        match self {
            SettingValue::Bool(b) => b.to_string(),
            SettingValue::Int(i) => i.to_string(),
            SettingValue::Text(t) => t.clone(),
        }
    }
}

impl SettingDef {
    /// A value as it's stored, or why it isn't one. Values end up in command lines and
    /// config files, so text can't hold line breaks or other control characters.
    pub fn normalize(&self, value: &str) -> std::result::Result<String, String> {
        let value = value.trim();
        match self.kind {
            SettingKind::Toggle => match value {
                "true" | "false" => Ok(value.to_string()),
                _ => Err("must be on or off".into()),
            },
            SettingKind::Number => {
                let n: i64 = value.parse().map_err(|_| "must be a whole number".to_string())?;
                if self.min.is_some_and(|min| n < min) || self.max.is_some_and(|max| n > max) {
                    let range = match (self.min, self.max) {
                        (Some(a), Some(b)) => format!("between {a} and {b}"),
                        (Some(a), None) => format!("{a} or more"),
                        (None, Some(b)) => format!("{b} or less"),
                        (None, None) => String::new(),
                    };
                    return Err(format!("must be {range}"));
                }
                Ok(n.to_string())
            }
            SettingKind::Choice => {
                if self.options.iter().any(|o| o == value) { Ok(value.to_string()) } else { Err(format!("must be one of {}", self.options.join(", "))) }
            }
            SettingKind::Text => {
                if value.chars().any(char::is_control) {
                    return Err("can't contain line breaks or control characters".into());
                }
                if value.chars().count() > 200 {
                    return Err("must be 200 characters or fewer".into());
                }
                Ok(value.to_string())
            }
        }
    }

    fn check_def(&self) -> std::result::Result<(), String> {
        if self.label.trim().is_empty() {
            return Err("needs a label".into());
        }
        if self.kind == SettingKind::Choice && self.options.is_empty() {
            return Err("a choice needs options".into());
        }
        if self.kind != SettingKind::Choice && !self.options.is_empty() {
            return Err("options are only for type: choice".into());
        }
        if self.kind != SettingKind::Number && (self.min.is_some() || self.max.is_some() || self.unit.is_some()) {
            return Err("min, max and unit are only for type: number".into());
        }
        if let (Some(min), Some(max)) = (self.min, self.max)
            && min > max {
                return Err("min is above max".into());
            }
        self.normalize(&self.default.text()).map(|_| ()).map_err(|e| format!("the default {e}"))
    }
}

/// A setting with its current value, for the service's card.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionInfo {
    pub name: String,
    pub label: String,
    pub description: Option<String>,
    pub kind: SettingKind,
    pub options: Vec<String>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub unit: Option<String>,
    pub default: String,
    pub value: String,
}

impl Manifest {
    /// A setting's value: what the user saved, else its default.
    pub fn setting(&self, settings: &Settings, name: &str) -> Option<String> {
        let def = self.settings.get(name)?;
        let saved = settings.service_options.get(&self.id).and_then(|o| o.get(name));
        // A saved value that no longer fits (the manifest changed) falls back to the default.
        Some(saved.and_then(|v| def.normalize(v).ok()).unwrap_or_else(|| def.default.text()))
    }

    pub fn options(&self, settings: &Settings) -> Vec<OptionInfo> {
        self.settings
            .iter()
            .map(|(name, d)| OptionInfo {
                name: name.clone(),
                label: d.label.clone(),
                description: d.description.clone(),
                kind: d.kind,
                options: d.options.clone(),
                min: d.min,
                max: d.max,
                unit: d.unit.clone(),
                default: d.default.text(),
                value: self.setting(settings, name).unwrap_or_default(),
            })
            .collect()
    }

    /// Checks and stores a setting in `settings` (the caller saves them). A value equal to
    /// the default is removed, so a later default change reaches the user.
    pub fn set_option(&self, settings: &mut Settings, name: &str, value: &str) -> Result<()> {
        let def = self.settings.get(name).ok_or_else(|| Error::msg(format!("{} has no setting named {name}", self.name)))?;
        let value = def.normalize(value).map_err(|e| Error::msg(format!("{}: {e}.", def.label)))?;
        let options = settings.service_options.entry(self.id.clone()).or_default();
        if value == def.default.text() {
            options.remove(name);
        } else {
            options.insert(name.to_string(), value);
        }
        if options.is_empty() {
            settings.service_options.remove(&self.id);
        }
        Ok(())
    }
}

/// One command-line argument, or several that apply only on some platforms.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Arg {
    Plain(String),
    Cond(CondArgs),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CondArgs {
    #[serde(default, rename = "if")]
    pub when: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub on: Vec<String>,
}

impl Arg {
    fn picked(&self) -> Vec<&str> {
        match self {
            Arg::Plain(s) => vec![s],
            Arg::Cond(c) if matches(&c.on) => c.value.iter().chain(&c.values).map(String::as_str).collect(),
            Arg::Cond(_) => vec![],
        }
    }

    fn all(&self) -> Vec<&str> {
        match self {
            Arg::Plain(s) => vec![s],
            Arg::Cond(c) => c.value.iter().chain(&c.values).map(String::as_str).collect(),
        }
    }
}

/// A YAML mapping that keeps its order (environment variables and .env lines read better
/// in the order they were written).
#[derive(Debug, Clone)]
pub struct Ordered<T>(pub Vec<(String, T)>);

impl<T> Default for Ordered<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> Ordered<T> {
    pub fn get(&self, key: &str) -> Option<&T> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn iter(&self) -> impl Iterator<Item = &(String, T)> {
        self.0.iter()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Ordered<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct V<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for V<T> {
            type Value = Ordered<T>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a map")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some((k, v)) = map.next_entry::<String, T>()? {
                    out.push((k, v));
                }
                Ok(Ordered(out))
            }
        }
        d.deserialize_map(V(PhantomData))
    }
}

/// Where downloads come from: one source, or a list where the first whose `on` matches
/// this platform wins (Redis: the community build on Windows, LampRabit's elsewhere).
#[derive(Debug, Clone)]
pub enum Sources {
    One(Source),
    Choice(Vec<Source>),
}

impl<'de> Deserialize<'de> for Sources {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        // Not `#[serde(untagged)]`: that would hide the real mistake behind "did not match
        // any variant".
        let value = serde_yaml_ng::Value::deserialize(d)?;
        if value.is_sequence() {
            serde_yaml_ng::from_value(value).map(Sources::Choice).map_err(serde::de::Error::custom)
        } else {
            serde_yaml_ng::from_value(value).map(Sources::One).map_err(serde::de::Error::custom)
        }
    }
}

impl Sources {
    pub fn all(&self) -> Vec<&Source> {
        match self {
            Sources::One(s) => vec![s],
            Sources::Choice(list) => list.iter().collect(),
        }
    }

    /// This platform's source.
    pub fn here(&self) -> Option<&Source> {
        self.all().into_iter().find(|s| matches(&s.on))
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    #[serde(default)]
    pub on: Vec<String>,
    /// LampRabit's own CI builds, from the `binaries` release's manifest (built-in only).
    #[serde(default)]
    pub lamprabit: bool,
    /// `owner/repo` on GitHub: versions (and files, unless `url` says otherwise) come from
    /// its releases or tags.
    #[serde(default)]
    pub github: Option<String>,
    /// A package on npm's registry: its latest version's tarball.
    #[serde(default)]
    pub npm: Option<String>,
    /// A vendor's JSON API that lists the releases (MariaDB, MongoDB, PyPI…).
    #[serde(default)]
    pub json: Option<crate::sources::JsonSource>,
    /// A web page or folder listing whose links are the downloads (Apache Lounge, nginx.org).
    #[serde(default)]
    pub page: Option<crate::sources::PageSource>,
    /// The download's archive type when no `assets` entry gives it; else from its address.
    #[serde(default)]
    pub archive: Option<Archive>,
    #[serde(default)]
    pub versions: Versions,
    /// The oldest major version offered (`per-major`).
    #[serde(default)]
    pub min: Option<u64>,
    /// How many lines to offer (`per-line`; 4 when not given).
    #[serde(default)]
    pub keep: Option<usize>,
    /// Only tags ending with one of these suffixes, which the version drops; the value is
    /// the label shown next to it (ClickHouse's `-lts`, `-stable`).
    #[serde(default)]
    pub tags: Ordered<String>,
    /// The file to download per platform (`windows-x64`, `windows`, `unix`); `{version}` is
    /// the release's version.
    #[serde(default)]
    pub assets: Ordered<Asset>,
    /// Download from here instead of the GitHub release (`{version}`, `{file}`).
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub checksum: Checksum,
    /// A note shown next to the versions ("Large download", "PostgreSQL {major}").
    #[serde(default)]
    pub label: Option<String>,
}

/// Which versions a GitHub source offers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Versions {
    /// The release GitHub marks as latest.
    #[default]
    Latest,
    /// The newest recent release that has this platform's file (a release sometimes
    /// ships without one platform's build).
    Newest,
    /// The newest release of each major version, from `min` on.
    PerMajor,
    /// The newest patch of each `major.minor` line, the `keep` newest lines.
    PerLine,
    /// The newest `x.y.z` tag (projects that tag releases without publishing them).
    NewestTag,
    /// Every release the list gives (a JSON API's stable lines, each its own entry).
    All,
}

/// How a download is checked. Nothing is installed unchecked: a missing checksum stops
/// the install.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Checksum {
    /// The SHA-256 GitHub reports for each release file.
    #[default]
    GithubDigest,
    /// npm's `dist.integrity` (SHA-512).
    Npm,
    /// A published SHA-256 file (`{url}` is the download's address).
    Sha256(String),
    /// A published SHA-512 file.
    Sha512(String),
    /// A published MD5 file (MySQL publishes nothing stronger; HTTPS covers the rest).
    Md5(String),
    /// None published (built-in manifests only; HTTPS covers the download).
    None,
}

// By hand: sources go through serde_yaml_ng::Value (for their list form), which would
// expect `!sha256` tags instead of `{ sha256: … }` maps.
impl<'de> Deserialize<'de> for Checksum {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Word(String),
            File {
                #[serde(default)]
                sha256: Option<String>,
                #[serde(default)]
                sha512: Option<String>,
                #[serde(default)]
                md5: Option<String>,
            },
        }
        use serde::de::Error as _;
        match Raw::deserialize(d)? {
            Raw::Word(w) => match w.as_str() {
                "github-digest" => Ok(Checksum::GithubDigest),
                "npm" => Ok(Checksum::Npm),
                "none" => Ok(Checksum::None),
                other => Err(D::Error::custom(format!("checksum \"{other}\" isn't one of github-digest, npm, none, {{ sha256: … }}, {{ sha512: … }}"))),
            },
            Raw::File { sha256: Some(t), sha512: None, md5: None } => Ok(Checksum::Sha256(t)),
            Raw::File { sha256: None, sha512: Some(t), md5: None } => Ok(Checksum::Sha512(t)),
            Raw::File { sha256: None, sha512: None, md5: Some(t) } => Ok(Checksum::Md5(t)),
            Raw::File { .. } => Err(D::Error::custom("checksum needs one of sha256, sha512 or md5")),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub file: String,
    pub archive: Archive,
    /// Files tried in order when `file` has no published checksum (MySQL's macOS builds
    /// carry the macOS version they're for, which moves on).
    #[serde(default)]
    pub fallbacks: Vec<String>,
}

impl Source {
    /// This platform's file: an exact `<os>-<arch>` key wins over an OS one, which wins
    /// over `unix`.
    pub fn asset(&self) -> Option<&Asset> {
        self.assets
            .get(&platform())
            .or_else(|| self.assets.get(os()))
            .or_else(|| if os() == "windows" { None } else { self.assets.get("unix") })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Need {
    pub runtime: String,
    /// Oldest version that works ("17" takes 17.x).
    #[serde(default)]
    pub min: Option<String>,
    /// Newest version that works, by prefix ("3.14" takes 3.14.x).
    #[serde(default)]
    pub max: Option<String>,
    /// The version an install picks when several fit.
    #[serde(default)]
    pub prefer: Option<String>,
    /// Only for this service's versions from `since` on and below `until` (Tomcat 10 needs
    /// Java 11, Tomcat 11 Java 17). Without a version (listing), only open-ended ones count.
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub until: Option<String>,
    #[serde(default)]
    pub on: Vec<String>,
    pub why: String,
}

impl Need {
    /// Whether it applies to this platform and to the service at `version`.
    pub fn applies(&self, version: Option<&str>) -> bool {
        let n = crate::version::numbers;
        matches(&self.on)
            && match version {
                None => self.until.is_none(),
                Some(v) => self.since.as_deref().is_none_or(|s| n(v) >= n(s)) && self.until.as_deref().is_none_or(|u| n(v) < n(u)),
            }
    }
}

/// Something LampRabit can't install, looked for on PATH.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemNeed {
    pub command: String,
    #[serde(default)]
    pub on: Vec<String>,
    /// How to install it, said when it's missing.
    pub hint: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortDef {
    pub default: u16,
    #[serde(default)]
    pub label: Option<String>,
    /// A few words on what uses it, under its field in Settings.
    #[serde(default)]
    pub hint: Option<String>,
}

/// One file operation before start; exactly one of `content`, `copy` or `merge`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileOp {
    /// Where to write, relative to etc/<id> unless it starts with a folder placeholder.
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    /// A file or folder to copy (into etc/<id>, or `to`).
    #[serde(default)]
    pub copy: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    /// A `key=value` file from the install, with these keys replaced.
    #[serde(default)]
    pub merge: Option<Merge>,
    /// Skip a copy whose source doesn't exist instead of failing.
    #[serde(default)]
    pub optional: bool,
    #[serde(default, rename = "if")]
    pub when: Option<String>,
    #[serde(default)]
    pub on: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Merge {
    pub from: String,
    pub set: Ordered<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub program: Text,
    #[serde(default)]
    pub args: Vec<Arg>,
    #[serde(default)]
    pub env: Ordered<Text>,
    /// The working folder; the install folder when not given.
    #[serde(default)]
    pub cwd: Option<Text>,
    /// Folders put in front of PATH.
    #[serde(default)]
    pub path: Vec<Arg>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stop {
    /// A command that asks the service to shut down cleanly (otherwise its processes end).
    pub command: Vec<Arg>,
    /// Only when this other runtime is installed (`{{runtime.<id>.home}}` points at it).
    #[serde(default)]
    pub if_installed: Option<String>,
}

/// How LampRabit asks whether the service answers: exactly one kind.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Health {
    #[serde(default)]
    pub http: Option<HttpCheck>,
    #[serde(default)]
    pub tcp: Option<PortRef>,
    #[serde(default)]
    pub line: Option<LineCheck>,
    /// MySQL's handshake (built-in manifests).
    #[serde(default)]
    pub mysql: Option<PortRef>,
    /// PostgreSQL's startup message (built-in manifests).
    #[serde(default)]
    pub postgres: Option<PortRef>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortRef {
    /// The port's name; the main port when not given.
    #[serde(default)]
    pub port: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpCheck {
    #[serde(default)]
    pub port: Option<String>,
    /// Any answer counts but a server error.
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineCheck {
    #[serde(default)]
    pub port: Option<String>,
    pub send: String,
    /// The reply starts with this.
    pub expect: String,
}

/// The health check, resolved to a port name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    Http(String, String),
    Tcp(String),
    Line(String, String, String),
    Mysql(String),
    Postgres(String),
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Behaviour {
    /// Needs a lot of memory or time: left out of Start all, the tray and start-on-launch.
    #[serde(default)]
    pub heavy: bool,
    /// Stays Starting until its health check passes.
    #[serde(default)]
    pub slow: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    pub url: String,
    #[serde(default)]
    pub env: Ordered<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ui {
    /// The name before the TLD: `n8n` for https://n8n.test.
    pub domain: String,
    #[serde(default)]
    pub port: Option<String>,
}

/// How apps connect, rendered for the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub url: String,
    pub env: Vec<(String, String)>,
}

// ---- platforms -------------------------------------------------------------------------

pub fn os() -> &'static str {
    std::env::consts::OS
}

pub fn arch() -> &'static str {
    if cfg!(target_arch = "aarch64") { "arm64" } else { "x64" }
}

/// This machine as a platform key: `windows-x64`, `macos-arm64`…
pub fn platform() -> String {
    format!("{}-{}", os(), arch())
}

const OS_WORDS: &[&str] = &["windows", "linux", "macos", "unix"];
const ARCH_WORDS: &[&str] = &["x64", "arm64"];

fn known_platform(word: &str) -> bool {
    if OS_WORDS.contains(&word) || ARCH_WORDS.contains(&word) {
        return true;
    }
    word.split_once('-').is_some_and(|(o, a)| OS_WORDS[..3].contains(&o) && ARCH_WORDS.contains(&a))
}

fn platform_matches(word: &str, os: &str, arch: &str) -> bool {
    match word {
        "unix" => os != "windows",
        w if OS_WORDS.contains(&w) => w == os,
        w if ARCH_WORDS.contains(&w) => w == arch,
        w => w == format!("{os}-{arch}"),
    }
}

/// Whether an `on:` list includes this machine (an empty one includes every machine).
pub fn matches(on: &[String]) -> bool {
    on.is_empty() || on.iter().any(|w| platform_matches(w, os(), arch()))
}

// ---- loading and checking ---------------------------------------------------------------

static BUILT_INS: OnceLock<Vec<Manifest>> = OnceLock::new();

/// The built-in manifests. They're checked by a test, so a broken one never ships.
pub fn built_ins() -> &'static [Manifest] {
    BUILT_INS.get_or_init(|| BUILT_IN.iter().map(|text| parse(text, true).unwrap_or_else(|e| panic!("built-in service manifest: {e}"))).collect())
}

/// Manifests that aren't built in (local files, later the registry), with whether they're
/// shown. Entries are never moved or dropped, so a `ServiceKind::Plugin`/`Runtime::Plugin`
/// index stays valid: an update replaces its entry, a removal hides it.
static EXTRAS: RwLock<Vec<(&'static Manifest, bool)>> = RwLock::new(Vec::new());

pub fn get(id: &str) -> Option<&'static Manifest> {
    built_ins().iter().find(|m| m.id == id).or_else(|| extras().into_iter().find(|m| m.id == id))
}

/// The shown non-built-in manifests.
pub fn extras() -> Vec<&'static Manifest> {
    EXTRAS.read().map(|e| e.iter().filter(|(_, shown)| *shown).map(|(m, _)| *m).collect()).unwrap_or_default()
}

/// The indexes of the shown non-built-in manifests.
pub fn extra_indexes() -> Vec<u16> {
    EXTRAS.read().map(|e| e.iter().enumerate().filter(|(_, (_, shown))| *shown).filter_map(|(i, _)| u16::try_from(i).ok()).collect()).unwrap_or_default()
}

pub fn extra(index: u16) -> &'static Manifest {
    EXTRAS.read().map(|e| e[usize::from(index)].0).expect("service list is unavailable")
}

/// Adds a non-built-in manifest, or replaces the one with its id, and returns its index. A
/// built-in's id is refused: two services would share folders and settings.
// shortcut: each added manifest is leaked to get a 'static reference; fine for the handful
// added by hand, revisit if manifests get reloaded often.
pub fn register(manifest: Manifest) -> Result<u16> {
    if !manifest.builtin.is_empty() {
        return Err(Error::msg(format!("{}: only built-in services may use builtin hooks", manifest.id)));
    }
    if built_ins().iter().any(|m| m.id == manifest.id) {
        return Err(Error::msg(format!("LampRabit already has a service with the id \"{}\". Pick another id.", manifest.id)));
    }
    let mut extras = EXTRAS.write().map_err(|_| Error::msg("service list is unavailable"))?;
    let entry = (&*Box::leak(Box::new(manifest)), true);
    if let Some(i) = extras.iter().position(|(m, _)| m.id == entry.0.id) {
        extras[i] = entry;
        return u16::try_from(i).map_err(|_| Error::msg("too many services"));
    }
    let index = u16::try_from(extras.len()).map_err(|_| Error::msg("too many services"))?;
    extras.push(entry);
    Ok(index)
}

/// Loads the services the user added (`<dir>/<id>/service.yaml`). A file that doesn't load
/// is skipped and described in the returned problems, so one bad file can't stop the rest.
pub fn load_local(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut problems = Vec::new();
    for entry in entries.flatten() {
        let file = entry.path().join("service.yaml");
        if !file.is_file() {
            continue;
        }
        let folder = entry.file_name().to_string_lossy().into_owned();
        let loaded = std::fs::read_to_string(&file).map_err(Error::from).and_then(|text| parse(&text, false)).and_then(|m| {
            // The folder is where its files live; an id that differs would point elsewhere.
            if m.id != folder {
                return Err(Error::msg(format!("its id \"{}\" doesn't match its folder \"{folder}\"", m.id)));
            }
            register(m)
        });
        if let Err(e) = loaded {
            problems.push(format!("The service in {} wasn't loaded: {e}", file.display()));
        }
    }
    problems
}

/// Hides a non-built-in service from every list; registering its id again shows it.
pub fn unregister(id: &str) {
    if let Ok(mut extras) = EXTRAS.write() {
        for (m, shown) in extras.iter_mut() {
            if m.id == id {
                *shown = false;
            }
        }
    }
}

/// Parses and checks a manifest. `builtin` allows the hooks only LampRabit's own may use.
pub fn parse(text: &str, builtin: bool) -> Result<Manifest> {
    let manifest: Manifest = serde_yaml_ng::from_str(text).map_err(|e| Error::msg(format!("Not a valid service manifest: {e}")))?;
    manifest.check(builtin).map_err(|e| Error::msg(format!("{}: {e}", manifest.id)))?;
    Ok(manifest)
}

/// A manifest's summary for approval (`Manifest::review`). Values keep their placeholders
/// (`{{port.http}}`, `{{data}}`): they're what the file says, not this machine's paths.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub id: String,
    pub name: String,
    pub blurb: String,
    pub category: String,
    pub homepage: Option<String>,
    pub revision: u32,
    /// Why it can't run on this platform, when it can't.
    pub unavailable: Option<String>,
    pub downloads: Option<String>,
    pub checksum: Option<String>,
    pub program: Option<String>,
    pub args: Vec<String>,
    /// Files it writes or copies before it starts.
    pub files: Vec<String>,
    /// Each port's label and default.
    pub ports: Vec<(String, u16)>,
    pub needs: Vec<String>,
}

/// Placeholders every manifest can use.
const PLAIN: &[&str] = &["home", "data", "etc", "logs", "tmp", "version", "major", "exe", "script"];
/// Placeholders that name a folder: a value starting with one is a path.
const FOLDERS: &[&str] = &["home", "data", "etc", "logs", "tmp"];

impl Manifest {
    pub fn uses(&self, hook: Hook) -> bool {
        self.builtin.contains(&hook)
    }

    fn check(&self, builtin: bool) -> std::result::Result<(), String> {
        if self.schema != SCHEMA {
            return Err(format!("schema {} isn't supported (this LampRabit reads schema {SCHEMA})", self.schema));
        }
        if let Some(needed) = &self.requires.lamprabit
            && crate::version::release(needed) > crate::version::release(env!("CARGO_PKG_VERSION")) {
                return Err(format!("needs LampRabit {needed} or newer; update LampRabit to use it"));
            }
        let id_ok = !self.id.is_empty()
            && self.id.len() <= 40
            && self.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !self.id.starts_with('-')
            && !self.id.ends_with('-');
        if !id_ok {
            return Err("id must be lowercase letters, digits and dashes".into());
        }
        if self.name.trim().is_empty() || self.blurb.trim().is_empty() {
            return Err("name and blurb can't be empty".into());
        }
        if !CATEGORIES.contains(&self.category.as_str()) {
            return Err(format!("category \"{}\" isn't one of {}", self.category, CATEGORIES.join(", ")));
        }
        if !builtin {
            if !self.builtin.is_empty() {
                return Err("builtin hooks are only for LampRabit's own services".into());
            }
            if self.health.mysql.is_some() || self.health.postgres.is_some() {
                return Err("health: mysql and postgres checks are only for LampRabit's own services".into());
            }
        }

        // Every platform word.
        let mut words: Vec<&str> = Vec::new();
        for t in [&self.origin, &self.unavailable, &self.config].into_iter().flatten() {
            words.extend(t.platforms());
        }
        words.extend(self.binary.platforms());
        for src in self.source.all() {
            words.extend(src.on.iter().map(String::as_str));
            words.extend(src.assets.iter().map(|(k, _)| k.as_str()));
        }
        words.extend(self.needs.iter().flat_map(|n| n.on.iter().map(String::as_str)));
        words.extend(self.system_needs.iter().flat_map(|n| n.on.iter().map(String::as_str)));
        words.extend(self.files.iter().flat_map(|f| f.on.iter().map(String::as_str)));
        let arg_words = |args: &[Arg]| -> Vec<String> {
            args.iter().filter_map(|a| if let Arg::Cond(c) = a { Some(c.on.clone()) } else { None }).flatten().collect()
        };
        let mut owned: Vec<String> = arg_words(&self.dirs);
        if let Some(run) = &self.run {
            owned.extend(arg_words(&run.args));
            owned.extend(arg_words(&run.path));
            owned.extend(run.program.platforms().into_iter().map(str::to_string));
            owned.extend(run.env.iter().flat_map(|(_, t)| t.platforms()).map(str::to_string));
        }
        if let Some(stop) = &self.stop {
            owned.extend(arg_words(&stop.command));
        }
        words.extend(owned.iter().map(String::as_str));
        if let Some(bad) = words.iter().find(|w| !known_platform(w)) {
            return Err(format!("unknown platform \"{bad}\" (use windows, linux, macos, unix, x64, arm64 or <os>-<arch>)"));
        }

        // Ports.
        let mut names: Vec<&str> = Vec::new();
        for (name, port) in self.ports.iter() {
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                return Err(format!("port name \"{name}\" must be lowercase letters and underscores"));
            }
            if names.contains(&name.as_str()) {
                return Err(format!("port \"{name}\" is listed twice"));
            }
            names.push(name);
            if !builtin && port.default < 1024 {
                return Err(format!("port \"{name}\": {} needs administrator rights on Linux and macOS; pick one above 1023", port.default));
            }
        }
        if names.is_empty() && !self.uses(Hook::Ports) {
            return Err("at least one port is needed".into());
        }
        let port_ok = |p: &Option<String>| p.as_deref().is_none_or(|p| names.contains(&p));
        let checks = [self.health.http.is_some(), self.health.tcp.is_some(), self.health.line.is_some(), self.health.mysql.is_some(), self.health.postgres.is_some()];
        if checks.iter().filter(|c| **c).count() != 1 {
            return Err("health needs exactly one of http, tcp, line, mysql, postgres".into());
        }
        let health_port = [
            self.health.http.as_ref().map(|h| &h.port),
            self.health.tcp.as_ref().map(|h| &h.port),
            self.health.line.as_ref().map(|h| &h.port),
            self.health.mysql.as_ref().map(|h| &h.port),
            self.health.postgres.as_ref().map(|h| &h.port),
        ];
        if health_port.into_iter().flatten().any(|p| !port_ok(p)) {
            return Err("health names a port that isn't in ports".into());
        }
        if let Some(ui) = &self.ui {
            let domain_ok = !ui.domain.is_empty() && ui.domain.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if !domain_ok {
                return Err("ui.domain must be lowercase letters, digits and dashes".into());
            }
            if !port_ok(&ui.port) {
                return Err("ui names a port that isn't in ports".into());
            }
        }

        // Sources.
        for src in self.source.all() {
            check_source(src, builtin)?;
        }
        for b in self.binary.all() {
            inside(b).map_err(|e| format!("binary: {e}"))?;
        }

        // Needs.
        for n in &self.needs {
            if !NEEDS.contains(&n.runtime.as_str()) {
                return Err(format!("needs: \"{}\" isn't something LampRabit can provide (one of {})", n.runtime, NEEDS.join(", ")));
            }
        }
        for n in &self.system_needs {
            if n.command.is_empty() || !n.command.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) {
                return Err(format!("system_needs: \"{}\" must be a command name", n.command));
            }
        }

        // Launch.
        if self.run.is_none() && !self.uses(Hook::Launch) {
            return Err("run is needed (program and args)".into());
        }
        if let Some(run) = &self.run {
            for program in run.program.all() {
                if !FOLDERS.iter().chain(&["needs."]).any(|f| program.starts_with(&format!("{{{{{f}"))) {
                    return Err(format!("run.program \"{program}\" must be inside the install ({{{{home}}}}/…) or a needed runtime ({{{{needs.<id>.bin}}}}/…)"));
                }
            }
            for (key, _) in run.env.iter() {
                if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    return Err(format!("run.env: \"{key}\" isn't a variable name"));
                }
            }
        }
        if let Some(stop) = &self.stop {
            let program = stop.command.first().and_then(|a| a.all().first().copied()).unwrap_or_default();
            if !["{{home}}", "{{needs.", "{{runtime."].iter().any(|p| program.starts_with(p)) {
                return Err(format!("stop.command's program \"{program}\" must be inside the install or a runtime's"));
            }
            if let Some(id) = &stop.if_installed
                && !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                    return Err(format!("stop.if_installed \"{id}\" isn't a runtime id"));
                }
        }

        // Files stay inside the service's own folders.
        for (i, f) in self.files.iter().enumerate() {
            let kinds = [f.content.is_some(), f.copy.is_some(), f.merge.is_some()];
            if kinds.iter().filter(|k| **k).count() != 1 {
                return Err(format!("files[{i}] needs exactly one of content, copy, merge"));
            }
            if (f.content.is_some() || f.merge.is_some()) && f.path.is_none() {
                return Err(format!("files[{i}] needs a path"));
            }
            for p in [&f.path, &f.to, &f.copy].into_iter().flatten().chain(f.merge.as_ref().map(|m| &m.from)) {
                inside(p).map_err(|e| format!("files[{i}]: {e}"))?;
            }
        }
        for d in &self.dirs {
            for p in d.all() {
                if !FOLDERS.iter().any(|f| p.starts_with(&format!("{{{{{f}}}}}"))) {
                    return Err(format!("dirs: \"{p}\" must start with a folder placeholder ({{{{data}}}}, {{{{logs}}}}…)"));
                }
                inside(p).map_err(|e| format!("dirs: {e}"))?;
            }
        }

        // Settings.
        let mut seen: Vec<&str> = Vec::new();
        for (name, def) in self.settings.iter() {
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
                return Err(format!("setting name \"{name}\" must be lowercase letters, digits and underscores"));
            }
            if seen.contains(&name.as_str()) {
                return Err(format!("setting \"{name}\" is listed twice"));
            }
            seen.push(name);
            def.check_def().map_err(|e| format!("setting \"{name}\": {e}"))?;
        }
        for when in self.conditions() {
            let name = when.trim_start_matches('!').strip_prefix("setting.").unwrap_or_default();
            if self.settings.get(name).is_none() {
                return Err(format!("if: \"{when}\" must be setting.<name> (or !setting.<name>) for a setting in settings"));
            }
        }

        // Placeholders.
        let mut allowed: Vec<String> = PLAIN.iter().map(|s| s.to_string()).collect();
        allowed.extend(self.settings.iter().map(|(n, _)| format!("setting.{n}")));
        allowed.extend(names.iter().map(|n| format!("port.{n}")));
        for n in &self.needs {
            allowed.push(format!("needs.{}.home", n.runtime));
            allowed.push(format!("needs.{}.bin", n.runtime));
        }
        if let Some(id) = self.stop.as_ref().and_then(|s| s.if_installed.as_ref()) {
            allowed.push(format!("runtime.{id}.home"));
        }
        if self.ui.is_some() {
            allowed.push("ui.url".into());
        }
        for text in self.texts() {
            for name in placeholders(text)? {
                if !allowed.contains(&name) {
                    return Err(format!("unknown placeholder {{{{{name}}}}}"));
                }
            }
        }
        Ok(())
    }

    fn conditions(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        fn args(args: &[Arg]) -> Vec<&str> {
            args.iter().filter_map(|a| if let Arg::Cond(c) = a { c.when.as_deref() } else { None }).collect()
        }
        fn text(t: &Text) -> Vec<&str> {
            match t {
                Text::Plain(_) => vec![],
                Text::One(c) => c.when.as_deref().into_iter().collect(),
                Text::Choice(list) => list.iter().filter_map(|c| c.when.as_deref()).collect(),
            }
        }
        out.extend(args(&self.dirs));
        out.extend(self.files.iter().filter_map(|f| f.when.as_deref()));
        if let Some(run) = &self.run {
            out.extend(args(&run.args));
            out.extend(args(&run.path));
            out.extend(text(&run.program));
            out.extend(run.cwd.iter().flat_map(text));
            out.extend(run.env.iter().flat_map(|(_, t)| text(t)));
        }
        if let Some(stop) = &self.stop {
            out.extend(args(&stop.command));
        }
        out
    }

    /// Every string that gets placeholders filled in.
    fn texts(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        out.extend(self.binary.all());
        out.extend(self.config.iter().flat_map(Text::all));
        out.extend(self.dirs.iter().flat_map(Arg::all));
        for f in &self.files {
            out.extend([&f.path, &f.content, &f.copy, &f.to].into_iter().flatten().map(String::as_str));
            if let Some(m) = &f.merge {
                out.push(&m.from);
                out.extend(m.set.iter().map(|(_, v)| v.as_str()));
            }
        }
        if let Some(run) = &self.run {
            out.extend(run.program.all());
            out.extend(run.args.iter().flat_map(Arg::all));
            out.extend(run.path.iter().flat_map(Arg::all));
            out.extend(run.cwd.iter().flat_map(Text::all));
            out.extend(run.env.iter().flat_map(|(_, t)| t.all()));
        }
        if let Some(stop) = &self.stop {
            out.extend(stop.command.iter().flat_map(Arg::all));
        }
        if let Some(c) = &self.connection {
            out.push(&c.url);
            out.extend(c.env.iter().map(|(_, v)| v.as_str()));
        }
        out
    }

    /// What a user approves before adding this service from a file: where it downloads from,
    /// how that's checked, what it runs and which files and ports it uses, on this platform.
    pub fn review(&self) -> Review {
        let source = self.source.here();
        let downloads = source.map(|s| match (&s.github, &s.npm, &s.url) {
            (_, _, Some(url)) => url.clone(),
            (Some(repo), _, None) => format!("GitHub releases of {repo}"),
            (None, Some(package), None) => format!("The npm package {package}"),
            _ => "Nowhere".to_string(),
        });
        let checksum = source.map(|s| match &s.checksum {
            Checksum::GithubDigest => "The SHA-256 GitHub reports for each file".to_string(),
            Checksum::Npm => "npm's SHA-512 for the package".to_string(),
            Checksum::Sha256(at) => format!("The SHA-256 published at {at}"),
            Checksum::Sha512(at) => format!("The SHA-512 published at {at}"),
            Checksum::Md5(at) => format!("The MD5 published at {at}"),
            Checksum::None => "Not checked".to_string(),
        });
        let run = self.run.as_ref();
        Review {
            id: self.id.clone(),
            name: self.name.clone(),
            blurb: self.blurb.clone(),
            category: self.category.clone(),
            homepage: self.homepage.clone(),
            revision: self.revision,
            unavailable: self.unavailable_here().map(str::to_string),
            downloads,
            checksum,
            program: run.and_then(|r| r.program.pick()).map(str::to_string),
            args: run.map(|r| r.args.iter().flat_map(Arg::picked).map(str::to_string).collect()).unwrap_or_default(),
            files: self.files.iter().filter(|f| matches(&f.on)).filter_map(|f| f.path.clone().or_else(|| f.to.clone()).or_else(|| f.copy.clone())).collect(),
            ports: self.ports.iter().map(|(name, d)| (d.label.clone().unwrap_or_else(|| name.clone()), d.default)).collect(),
            needs: self.needs.iter().filter(|n| matches(&n.on)).map(|n| match &n.min {
                Some(min) => format!("{} {min} or newer", n.runtime),
                None => n.runtime.clone(),
            }).collect(),
        }
    }

    /// `binary` relative to the install folder: the runtime's "install is complete" marker.
    pub fn marker(&self) -> PathBuf {
        let b = self.binary.pick().unwrap_or_default();
        let b = b.trim_start_matches("{{home}}").trim_start_matches(['/', '\\']);
        PathBuf::from(b.replace("{{exe}}", if cfg!(windows) { ".exe" } else { "" }).replace("{{script}}", if cfg!(windows) { ".bat" } else { "" }))
    }

    /// The main port's name.
    pub fn main_port(&self) -> Option<&str> {
        self.ports.0.first().map(|(n, _)| n.as_str())
    }

    pub fn check_kind(&self) -> Check {
        let main = || self.main_port().unwrap_or_default().to_string();
        let port = |p: &Option<String>| p.clone().unwrap_or_else(main);
        let h = &self.health;
        if let Some(c) = &h.http {
            Check::Http(port(&c.port), c.path.clone())
        } else if let Some(c) = &h.line {
            Check::Line(port(&c.port), c.send.clone(), c.expect.clone())
        } else if let Some(c) = &h.mysql {
            Check::Mysql(port(&c.port))
        } else if let Some(c) = &h.postgres {
            Check::Postgres(port(&c.port))
        } else {
            Check::Tcp(port(&h.tcp.clone().unwrap_or_default().port))
        }
    }

    pub fn log_file(&self) -> String {
        self.log.clone().unwrap_or_else(|| format!("{}-output.log", self.id))
    }

    /// Why it can't run on this machine, when it can't.
    pub fn unavailable_here(&self) -> Option<&str> {
        self.unavailable.as_ref().and_then(Text::pick)
    }

    /// A port's number from the settings.
    pub fn port(&self, settings: &Settings, name: &str) -> Option<u16> {
        let def = self.ports.get(name)?;
        Some(settings.ports.services.get(&format!("{}.{name}", self.id)).copied().unwrap_or(def.default))
    }

    /// Gives each port of a service that has none yet its default, or the next
    /// port no other service is set to use and `free` says is free right now.
    pub fn assign_ports(&self, settings: &mut Settings, free: impl Fn(u16) -> bool) {
        let mut taken: Vec<u16> = serde_json::to_value(&settings.ports)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .into_iter()
            .flatten()
            .filter_map(|(_, v)| v.as_u64().and_then(|p| u16::try_from(p).ok()))
            .collect();
        // shortcut: PHP workers take blocks of 10 from php_fpm, one per PHP version in use;
        // 100 covers ten versions, widen it if more ever run at once.
        taken.extend(settings.ports.php_fpm..settings.ports.php_fpm.saturating_add(100));
        taken.extend(settings.ports.services.iter().filter(|(k, _)| !k.starts_with(&format!("{}.", self.id))).map(|(_, p)| *p));
        for m in built_ins().iter().chain(extras()).filter(|m| m.id != self.id) {
            taken.extend(m.port_numbers(settings));
        }
        for (name, def) in self.ports.iter() {
            let key = format!("{}.{name}", self.id);
            if settings.ports.services.contains_key(&key) {
                continue;
            }
            let Some(port) = (def.default..=u16::MAX).find(|p| !taken.contains(p) && free(*p)) else { continue };
            taken.push(port);
            settings.ports.services.insert(key, port);
        }
    }

    /// Every port it listens on, the main one first.
    pub fn port_numbers(&self, settings: &Settings) -> Vec<u16> {
        self.ports.iter().filter_map(|(n, _)| self.port(settings, n)).collect()
    }
}

fn check_source(src: &Source, builtin: bool) -> std::result::Result<(), String> {
    let kinds = [src.lamprabit, src.github.is_some(), src.npm.is_some(), src.json.is_some(), src.page.is_some()];
    if kinds.iter().filter(|k| **k).count() != 1 {
        return Err("each source needs exactly one of github, npm, json or page (lamprabit is LampRabit's own)".into());
    }
    crate::sources::check(src)?;
    let vendor = src.json.is_some() || src.page.is_some();
    let inline = src.json.as_ref().is_some_and(|j| j.sha256.is_some() || j.sha512.is_some());
    if vendor && !inline && matches!(src.checksum, Checksum::GithubDigest | Checksum::Npm) {
        return Err("source: a json or page download needs its checksum: json.sha256/sha512 from the answer, or checksum: { sha256 | sha512 | md5: <file> }".into());
    }
    if !builtin {
        if src.lamprabit {
            return Err("source: lamprabit is only for LampRabit's own services".into());
        }
        if src.checksum == Checksum::None {
            return Err("source: every download needs a checksum (checksum: none is only for LampRabit's own services)".into());
        }
    }
    let https = |what: &str, t: &str| {
        if t.starts_with("https://") || t.starts_with("{url}") { Ok(()) } else { Err(format!("source.{what} \"{t}\" must be an https:// address")) }
    };
    if let Some(url) = &src.url {
        https("url", url)?;
    }
    match &src.checksum {
        Checksum::Sha256(t) | Checksum::Sha512(t) | Checksum::Md5(t) => https("checksum", t)?,
        Checksum::GithubDigest if src.github.is_some() && src.url.is_some() => {
            return Err("source: a download from url needs checksum: { sha256: … } or { sha512: … } (GitHub's digests only cover its own files)".into());
        }
        Checksum::GithubDigest if src.npm.is_some() => return Err("source: npm packages use checksum: npm".into()),
        Checksum::Npm if src.npm.is_none() => return Err("source: checksum: npm is only for npm packages".into()),
        _ => {}
    }
    if let Some(repo) = &src.github {
        if repo.split('/').count() != 2 || repo.split('/').any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))) {
            return Err(format!("source.github \"{repo}\" must be owner/repo"));
        }
        if src.assets.0.is_empty() {
            return Err("source.assets lists no files".into());
        }
        if let Some((_, a)) = src.assets.iter().find(|(_, a)| a.file.is_empty() || a.file.contains('/') || a.file.contains('\\')) {
            return Err(format!("source asset \"{}\" must be a file name", a.file));
        }
    }
    if let Some(name) = &src.npm
        && (name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "-_.@/".contains(c))) {
            return Err(format!("source.npm \"{name}\" isn't a package name"));
        }
    if src.github.is_none() && (!src.tags.0.is_empty() || src.url.is_some()) {
        return Err("source: tags and url are for github sources".into());
    }
    if !vendor && src.github.is_none() && (src.versions != Versions::Latest || !src.assets.0.is_empty()) {
        return Err("source: versions and assets are for github, json and page sources".into());
    }
    Ok(())
}

/// What `needs:` can name: runtimes whose folder a service can be pointed at.
const NEEDS: &[&str] = &["java", "erlang", "node", "python", "postgres"];

/// A path must not climb out of the folder it starts in.
fn inside(path: &str) -> std::result::Result<(), String> {
    let placeholder = path.starts_with("{{");
    let rest = if placeholder { path.split_once("}}").map(|(_, r)| r).unwrap_or_default() } else { path };
    if rest.split(['/', '\\']).any(|part| part == "..") {
        return Err(format!("\"{path}\" leaves its folder (..)"));
    }
    if !placeholder && (path.starts_with(['/', '\\']) || path.contains(':')) {
        return Err(format!("\"{path}\" must be relative or start with a folder placeholder"));
    }
    Ok(())
}

/// The `{{ name }}` placeholders in a text (with filters like `|slash` taken off).
fn placeholders(text: &str) -> std::result::Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let end = after.find("}}").ok_or_else(|| format!("unclosed {{{{ in \"{text}\""))?;
        let inner = after[..end].trim();
        let (name, filter) = inner.split_once('|').map(|(n, f)| (n.trim(), Some(f.trim()))).unwrap_or((inner, None));
        if let Some(f) = filter
            && f != "slash" {
                return Err(format!("unknown filter |{f} (only |slash)"));
            }
        out.push(name.to_string());
        rest = &after[end + 2..];
    }
    Ok(out)
}

// ---- rendering ---------------------------------------------------------------------------

/// Fills in a manifest's placeholders for one installed version.
pub struct Ctx<'a> {
    pub manifest: &'a Manifest,
    settings: &'a Settings,
    layout: &'a Layout,
    pub version: String,
    /// Values computed outside (`ui.url`, `needs.<id>.home`…), by placeholder name.
    extra: BTreeMap<String, String>,
}

impl<'a> Ctx<'a> {
    pub fn new(manifest: &'a Manifest, settings: &'a Settings, layout: &'a Layout) -> Result<Self> {
        let version = settings.versions.get(&manifest.id).cloned().ok_or_else(|| Error::msg(format!("{} is not installed", manifest.name)))?;
        Ok(Self { manifest, settings, layout, version, extra: BTreeMap::new() })
    }

    /// For what doesn't need an install (paths shown before the first one, connection
    /// details): the active version when there is one.
    pub fn any(manifest: &'a Manifest, settings: &'a Settings, layout: &'a Layout) -> Self {
        let version = settings.versions.get(&manifest.id).cloned().unwrap_or_default();
        Self { manifest, settings, layout, version, extra: BTreeMap::new() }
    }

    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.extra.insert(name.into(), value.into());
    }

    pub fn home(&self) -> PathBuf {
        self.layout.bin(&self.manifest.id, &self.version)
    }

    fn major(&self) -> &str {
        self.version.split('.').next().unwrap_or_default()
    }

    pub fn data(&self) -> PathBuf {
        let id = &self.manifest.id;
        match self.manifest.data {
            DataMode::Shared => self.layout.data(id),
            DataMode::PerMajor => self.layout.data(&format!("{id}-{}", self.major())),
            DataMode::Storage => self.settings.storage_dir(self.layout, id),
        }
    }

    pub fn etc(&self) -> PathBuf {
        self.layout.etc(&self.manifest.id)
    }

    fn folder(&self, name: &str) -> Option<PathBuf> {
        Some(match name {
            "home" => self.home(),
            "data" => self.data(),
            "etc" => self.etc(),
            "logs" => self.layout.logs().join(&self.manifest.id),
            "tmp" => self.layout.tmp().join(&self.manifest.id),
            _ => return None,
        })
    }

    fn value(&self, name: &str) -> Result<String> {
        if let Some(dir) = self.folder(name) {
            return Ok(dir.to_string_lossy().into_owned());
        }
        if let Some(v) = self.extra.get(name) {
            return Ok(v.clone());
        }
        Ok(match name {
            "version" => self.version.clone(),
            "major" => self.major().to_string(),
            "exe" => if cfg!(windows) { ".exe" } else { "" }.to_string(),
            "script" => if cfg!(windows) { ".bat" } else { "" }.to_string(),
            _ => {
                if let Some(setting) = name.strip_prefix("setting.") {
                    return self.manifest.setting(self.settings, setting).ok_or_else(|| Error::msg(format!("{}: no setting named {setting}", self.manifest.id)));
                }
                if let Some(port) = name.strip_prefix("port.") {
                    return self.manifest.port(self.settings, port).map(|p| p.to_string()).ok_or_else(|| Error::msg(format!("{}: no port named {port}", self.manifest.id)));
                }
                return Err(Error::msg(format!("{}: {{{{{name}}}}} has no value here", self.manifest.id)));
            }
        })
    }

    /// Fills in the placeholders. What follows a folder placeholder up to the next space is
    /// a path (`{{etc}}/my.ini`, `--dir={{data}}/x`): its `/` become this OS's separator,
    /// unless the placeholder has `|slash` (config files and MSYS programs want `/`).
    pub fn render(&self, text: &str) -> Result<String> {
        let mut out = String::new();
        let mut rest = text;
        let mut native = false;
        let push = |out: &mut String, literal: &str, native: &mut bool| {
            for ch in literal.chars() {
                if ch.is_whitespace() {
                    *native = false;
                }
                out.push(if *native && ch == '/' { std::path::MAIN_SEPARATOR } else { ch });
            }
        };
        while let Some(start) = rest.find("{{") {
            push(&mut out, &rest[..start], &mut native);
            let after = &rest[start + 2..];
            let end = after.find("}}").ok_or_else(|| Error::msg(format!("unclosed {{{{ in \"{text}\"")))?;
            let inner = after[..end].trim();
            let (name, slash) = match inner.split_once('|') {
                Some((n, _)) => (n.trim(), true),
                None => (inner, false),
            };
            let value = self.value(name)?;
            let is_folder = FOLDERS.contains(&name) || name.starts_with("needs.") || name.starts_with("runtime.");
            native = is_folder && !slash;
            out.push_str(&if slash { value.replace('\\', "/") } else { value });
            rest = &after[end + 2..];
        }
        push(&mut out, rest, &mut native);
        Ok(out)
    }

    pub fn path(&self, text: &str) -> Result<PathBuf> {
        let rendered = self.render(text)?;
        let path = PathBuf::from(&rendered);
        // Relative paths are inside etc/<id>.
        Ok(if path.is_absolute() { path } else { self.etc().join(rendered) })
    }

    /// Whether an `if:` holds: a toggle that's on, or any other setting that isn't empty.
    pub fn holds(&self, when: Option<&str>) -> bool {
        let Some(when) = when else { return true };
        let (negate, name) = match when.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, when),
        };
        let value = name.strip_prefix("setting.").and_then(|n| self.manifest.setting(self.settings, n)).unwrap_or_default();
        let on = !value.is_empty() && value != "false" && value != "0";
        on != negate
    }

    /// The value of a text for this platform and these settings.
    pub fn pick<'t>(&self, text: &'t Text) -> Option<&'t str> {
        match text {
            Text::Plain(s) => Some(s),
            Text::One(c) => (matches(&c.on) && self.holds(c.when.as_deref())).then_some(c.value.as_str()),
            Text::Choice(list) => list.iter().find(|c| matches(&c.on) && self.holds(c.when.as_deref())).map(|c| c.value.as_str()),
        }
    }

    pub fn text(&self, text: &Text) -> Result<Option<String>> {
        self.pick(text).map(|t| self.render(t)).transpose()
    }

    pub fn args(&self, args: &[Arg]) -> Result<Vec<String>> {
        let picked = args.iter().flat_map(|a| match a {
            Arg::Cond(c) if !self.holds(c.when.as_deref()) => vec![],
            a => a.picked(),
        });
        picked.map(|a| self.render(a)).collect()
    }

    /// The file whose presence means the install is complete.
    pub fn binary(&self) -> Option<PathBuf> {
        self.manifest.binary.pick().map(|b| {
            let p = self.render(b).unwrap_or_default();
            if Path::new(&p).is_absolute() { PathBuf::from(p) } else { self.home().join(p) }
        })
    }

    /// Creates the folders and writes the files a service needs before it starts.
    pub fn prepare(&self) -> Result<()> {
        for dir in self.args(&self.manifest.dirs)? {
            std::fs::create_dir_all(&dir).map_err(|e| Error::msg(format!("Couldn't create {}'s folder {dir}: {e}", self.manifest.name)))?;
        }
        for f in self.manifest.files.iter().filter(|f| matches(&f.on) && self.holds(f.when.as_deref())) {
            if let Some(content) = &f.content {
                let target = self.path(f.path.as_deref().unwrap_or_default())?;
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&target, self.render(content)?)?;
            } else if let Some(from) = &f.copy {
                let from = self.path(from)?;
                let to = match &f.to {
                    Some(to) => self.path(to)?,
                    None if from.is_dir() => self.etc(),
                    None => self.etc().join(from.file_name().unwrap_or_default()),
                };
                if !from.exists() {
                    if f.optional {
                        continue;
                    }
                    return Err(Error::msg(format!("{}'s install has no {}: reinstall it on the Services page.", self.manifest.name, from.display())));
                }
                copy(&from, &to)?;
            } else if let Some(merge) = &f.merge {
                let target = self.path(f.path.as_deref().unwrap_or_default())?;
                let shipped = std::fs::read_to_string(self.path(&merge.from)?).unwrap_or_default();
                let set: Vec<(String, String)> = merge.set.iter().map(|(k, v)| Ok((k.clone(), self.render(v)?))).collect::<Result<_>>()?;
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(target, merge_keys(&shipped, &set))?;
            }
        }
        Ok(())
    }

    pub fn connection(&self) -> Option<ConnectionInfo> {
        let c = self.manifest.connection.as_ref()?;
        Some(ConnectionInfo {
            url: self.render(&c.url).ok()?,
            env: c.env.iter().filter_map(|(k, v)| Some((k.clone(), self.render(v).ok()?))).collect(),
        })
    }
}

/// A `key=value` file without the keys LampRabit sets, then LampRabit's values.
fn merge_keys(shipped: &str, set: &[(String, String)]) -> String {
    let mut out: String = shipped
        .lines()
        .filter(|l| !set.iter().any(|(k, _)| l.trim_start().starts_with(&format!("{k}="))))
        .map(|l| format!("{l}\n"))
        .collect();
    out.push_str("\n# Added by LampRabit; regenerated on every start.\n");
    for (k, v) in set {
        out.push_str(&format!("{k}={v}\n"));
    }
    out
}

/// Copies a file, or a folder's files and subfolders, replacing what's there.
fn copy(from: &Path, to: &Path) -> Result<()> {
    if from.is_file() {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(from, to)?;
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)?.flatten() {
        copy(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
}

/// The connection details of a service that's installed, rendered with its settings.
pub fn connection(id: &str, settings: &Settings, layout: &Layout) -> Option<ConnectionInfo> {
    Ctx::any(get(id)?, settings, layout).connection()
}

// ---- catalogs ----------------------------------------------------------------------------

/// A release found for this platform from a manifest's source.
#[derive(Debug, Clone)]
pub struct Found {
    pub version: String,
    pub url: String,
    pub archive: Archive,
    pub sha256: Option<String>,
    pub sha512: Option<String>,
    pub md5: Option<String>,
    /// A published checksum file still to fetch: (which hash, its address).
    pub checksum_file: Option<(Hash, String)>,
    pub label: Option<String>,
}

/// Which hash a checksum file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hash {
    Sha256,
    Sha512,
    Md5,
}

impl Hash {
    /// Its length in hex digits, to find it in a checksum file of any layout.
    pub fn hex_len(self) -> usize {
        match self {
            Hash::Sha256 => 64,
            Hash::Sha512 => 128,
            Hash::Md5 => 32,
        }
    }
}

/// The address a GitHub or npm source's versions are read from.
pub fn catalog_url(src: &Source) -> Option<String> {
    if let Some(name) = &src.npm {
        return Some(format!("https://registry.npmjs.org/{name}/latest"));
    }
    let repo = src.github.as_ref()?;
    Some(match src.versions {
        Versions::Latest if src.tags.0.is_empty() => format!("https://api.github.com/repos/{repo}/releases/latest"),
        Versions::NewestTag => format!("https://api.github.com/repos/{repo}/tags?per_page=100"),
        _ => format!("https://api.github.com/repos/{repo}/releases?per_page=100"),
    })
}

pub fn no_download(m: &Manifest) -> Error {
    Error::msg(m.unavailable_here().map(str::to_string).unwrap_or_else(|| format!("{} has no download for {}.", m.name, platform())))
}

/// The releases a source offers this platform, from what `catalog_url` returned.
pub fn select(m: &Manifest, src: &Source, json: &serde_json::Value) -> Result<Vec<Found>> {
    if src.npm.is_some() {
        return select_npm(m, src, json);
    }
    let asset = src.asset().ok_or_else(|| no_download(m))?;
    // (version, the label its tag gives, the release).
    let mut candidates: Vec<(String, Option<String>, &serde_json::Value)> = Vec::new();
    let list: Vec<&serde_json::Value> = json.as_array().map(|l| l.iter().collect()).unwrap_or_else(|| vec![json]);
    if src.versions == Versions::NewestTag {
        let newest = list
            .iter()
            .filter_map(|t| t["name"].as_str())
            .map(|n| n.trim_start_matches('v'))
            .filter(|n| n.split('.').count() == 3 && n.split('.').all(|p| p.parse::<u64>().is_ok()))
            .max_by_key(|n| crate::version::numbers(n));
        candidates.extend(newest.map(|v| (v.to_string(), None, &serde_json::Value::Null)));
    } else {
        for release in list {
            if release["draft"] == true {
                continue;
            }
            let tag = release["tag_name"].as_str().unwrap_or_default();
            let (tag, label) = if src.tags.0.is_empty() {
                if release["prerelease"] == true {
                    continue;
                }
                (tag, None)
            } else {
                // The suffix decides (some projects mark stable releases as pre-releases).
                match src.tags.iter().find(|(suffix, _)| tag.ends_with(suffix.as_str())) {
                    Some((suffix, label)) => (&tag[..tag.len() - suffix.len()], Some(label.clone())),
                    None => continue,
                }
            };
            candidates.push((tag.trim_start_matches('v').to_string(), label, release));
        }
    }

    let key = |f: &Found| crate::version::numbers(&f.version);
    let mut found: Vec<Found> = Vec::new();
    // A release GitHub reports no digest for is left out (old uploads have none).
    let mut unchecked = None;
    for (version, tag_label, release) in candidates {
        let file = asset.file.replace("{version}", &version);
        let (url, sha256) = match &src.url {
            Some(url) => (url.replace("{version}", &version).replace("{file}", &file), None),
            None => {
                let Some(a) = release["assets"].as_array().into_iter().flatten().find(|a| a["name"] == file.as_str()) else { continue };
                let digest = a["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).map(str::to_string);
                (a["browser_download_url"].as_str().unwrap_or_default().to_string(), digest)
            }
        };
        let fill = |t: &str| t.replace("{url}", &url).replace("{version}", &version).replace("{file}", &file);
        let checksum_file = match &src.checksum {
            Checksum::GithubDigest if sha256.is_none() => {
                unchecked = Some(format!("GitHub reports no checksum for {file} ({} {version}), so LampRabit won't install it.", m.name));
                continue;
            }
            Checksum::Sha256(t) => Some((Hash::Sha256, fill(t))),
            Checksum::Sha512(t) => Some((Hash::Sha512, fill(t))),
            Checksum::Md5(t) => Some((Hash::Md5, fill(t))),
            _ => None,
        };
        let major = crate::version::numbers(&version).first().copied().unwrap_or_default();
        if src.versions == Versions::PerMajor && src.min.is_some_and(|min| major < min) {
            continue;
        }
        let label = tag_label.or_else(|| src.label.as_ref().map(|l| l.replace("{major}", &major.to_string()).replace("{version}", &version)));
        let candidate = Found { version, url, archive: asset.archive, sha256, sha512: None, md5: None, checksum_file, label };
        match src.versions {
            Versions::Latest | Versions::Newest | Versions::NewestTag | Versions::All => {
                found.push(candidate);
                if src.versions != Versions::All {
                    break;
                }
            }
            Versions::PerMajor | Versions::PerLine => {
                let parts = if src.versions == Versions::PerMajor { 1 } else { 2 };
                let line = |f: &Found| key(f).into_iter().take(parts).collect::<Vec<_>>();
                match found.iter_mut().find(|f| line(f) == line(&candidate)) {
                    Some(f) if key(&candidate) > key(f) => *f = candidate,
                    Some(_) => {}
                    None => found.push(candidate),
                }
            }
        }
    }
    found.sort_by_key(|f| std::cmp::Reverse(key(f)));
    if src.versions == Versions::PerLine {
        found.truncate(src.keep.unwrap_or(4));
    }
    if found.is_empty() {
        return Err(Error::msg(unchecked.unwrap_or_else(|| format!("No recent {} release has a download for {} ({}).", m.name, platform(), asset.file))));
    }
    Ok(found)
}

/// npm's latest version: its tarball, checked against `dist.integrity` (SHA-512, base64).
fn select_npm(m: &Manifest, src: &Source, json: &serde_json::Value) -> Result<Vec<Found>> {
    use base64::Engine;
    let version = json["version"].as_str().ok_or_else(|| Error::msg(format!("Couldn't read {}'s latest release from npm", m.name)))?.to_string();
    let sha512 = json["dist"]["integrity"]
        .as_str()
        .and_then(|i| i.strip_prefix("sha512-"))
        .and_then(|b| base64::engine::general_purpose::STANDARD.decode(b).ok())
        .map(|bytes| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>());
    if sha512.is_none() {
        return Err(Error::msg(format!("npm publishes no SHA-512 for {} {version}, so LampRabit won't install it.", m.name)));
    }
    let node = json["engines"]["node"].as_str().unwrap_or_default();
    // "Node {node}" only when the package says which Node it needs.
    let label = src.label.as_ref().filter(|l| !l.contains("{node}") || !node.is_empty()).map(|l| l.replace("{node}", node).replace("{version}", &version));
    Ok(vec![Found {
        url: json["dist"]["tarball"].as_str().unwrap_or_default().to_string(),
        version,
        archive: Archive::File,
        sha256: None,
        sha512,
        md5: None,
        checksum_file: None,
        label,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_manifests_are_valid() {
        let all = built_ins();
        assert_eq!(all.len(), BUILT_IN.len());
        let mut ids: Vec<&str> = all.iter().map(|m| m.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), all.len(), "two built-in manifests share an id");
    }

    #[test]
    fn a_manifest_for_a_newer_lamprabit_is_refused() {
        let e = with("requires: { lamprabit: \"99.0.0\" }\n").unwrap_err();
        assert!(e.contains("needs LampRabit 99.0.0"), "{e}");
        assert!(with("requires: { lamprabit: \"0.1.0\" }\n").is_ok());
    }

    #[test]
    fn a_file_without_a_digest_is_refused() {
        let (m, src) = source(|_| {});
        assert!(select(&m, &src, &release(&m, "v1.0.0", None)).unwrap_err().to_string().contains("no checksum"));
    }

    #[test]
    fn platform_words() {
        assert!(platform_matches("unix", "linux", "x64"));
        assert!(!platform_matches("unix", "windows", "x64"));
        assert!(platform_matches("macos-arm64", "macos", "arm64"));
        assert!(!platform_matches("macos-arm64", "macos", "x64"));
        assert!(platform_matches("arm64", "linux", "arm64"));
        assert!(known_platform("linux-arm64"));
        assert!(!known_platform("bsd"));
        assert!(!known_platform("unix-x64"));
    }

    const SMALL: &str = r#"
schema: 1
id: echo
name: Echo
category: tools
blurb: Says things back
source:
  github: someone/echo
  assets:
    windows: { file: "echo-{version}-windows.zip", archive: zip }
    linux-x64: { file: "echo-{version}-linux.tar.gz", archive: targz }
    linux-arm64: { file: "echo-{version}-linux-arm.tar.gz", archive: targz }
    macos: { file: "echo-{version}-macos.tar.gz", archive: targz }
binary: "echo{{exe}}"
ports:
  http: { default: 7000 }
run:
  program: "{{home}}/echo{{exe}}"
  args: [--port, "{{port.http}}", --data, "{{data}}/db", { values: [--unix, "yes"], on: [unix] }]
health: { http: { path: /health } }
connection:
  url: "http://127.0.0.1:{{port.http}}"
  env: { ECHO_URL: "http://127.0.0.1:{{port.http}}", ECHO_KEY: "" }
"#;

    fn with(extra: &str) -> std::result::Result<Manifest, String> {
        parse(&format!("{SMALL}{extra}"), false).map_err(|e| e.to_string())
    }

    #[test]
    fn a_community_manifest_parses_and_renders() {
        let m = with("").unwrap();
        assert_eq!(m.main_port(), Some("http"));
        assert_eq!(m.check_kind(), Check::Http("http".into(), "/health".into()));
        let mut settings = Settings::default();
        settings.versions.set("echo", Some("1.2.3".into()));
        let layout = Layout::new(std::env::temp_dir().join("lr-manifest-test"));
        let ctx = Ctx::new(&m, &settings, &layout).unwrap();
        let args = ctx.args(&m.run.as_ref().unwrap().args).unwrap();
        assert_eq!(args[1], "7000");
        assert_eq!(PathBuf::from(&args[3]), layout.data("echo").join("db"));
        assert_eq!(args.len(), if cfg!(windows) { 4 } else { 6 });
        assert_eq!(ctx.connection().unwrap().env, vec![("ECHO_URL".to_string(), "http://127.0.0.1:7000".to_string()), ("ECHO_KEY".to_string(), String::new())]);
        assert_eq!(ctx.render("{{data|slash}}/x").unwrap(), format!("{}/x", layout.data("echo").to_string_lossy().replace('\\', "/")));
    }

    #[test]
    fn community_manifests_cant_reach_outside_or_use_hooks() {
        let bad = |extra: &str, says: &str| {
            let e = with(extra).unwrap_err();
            assert!(e.contains(says), "{extra}: {e}");
        };
        bad("builtin: [launch]\n", "builtin hooks");
        bad("files:\n  - { path: ../../evil, content: x }\n", "leaves its folder");
        bad("files:\n  - { path: \"{{home}}/../x\", content: x }\n", "leaves its folder");
        bad("files:\n  - { path: \"C:/Windows/x\", content: x }\n", "relative");
        bad("dirs: [/etc/x]\n", "folder placeholder");
        bad("log: x\nunavailable: { value: no, on: [bsd] }\n", "unknown platform");
        bad("suggests: []\ndata: shared\nconfig: \"{{nope}}\"\n", "unknown placeholder");
        bad("needs:\n  - { runtime: dotnet, why: x }\n", "isn't something");
    }

    #[test]
    fn programs_must_come_from_the_install() {
        let text = SMALL.replace("program: \"{{home}}/echo{{exe}}\"", "program: /bin/sh");
        assert!(parse(&text, false).unwrap_err().to_string().contains("inside the install"));
    }

    #[test]
    fn low_ports_are_refused_for_community_manifests() {
        let text = SMALL.replace("default: 7000", "default: 80");
        assert!(parse(&text, false).unwrap_err().to_string().contains("above 1023"));
    }

    #[test]
    fn health_needs_exactly_one_check() {
        let text = SMALL.replace("health: { http: { path: /health } }", "health: { http: { path: / }, tcp: {} }");
        assert!(parse(&text, false).unwrap_err().to_string().contains("exactly one"));
        let text = SMALL.replace("health: { http: { path: /health } }", "health: { tcp: { port: nope } }");
        assert!(parse(&text, false).unwrap_err().to_string().contains("isn't in ports"));
    }

    /// The echo manifest's source, changed by `f`.
    fn source(f: impl FnOnce(&mut Source)) -> (Manifest, Source) {
        let m = with("").unwrap();
        let mut src = m.source.here().unwrap().clone();
        f(&mut src);
        (m, src)
    }

    /// A GitHub release with this platform's echo file.
    fn release(m: &Manifest, tag: &str, digest: Option<&str>) -> serde_json::Value {
        let version = tag.trim_start_matches('v');
        let name = m.source.here().unwrap().asset().unwrap().file.replace("{version}", version);
        let mut asset = serde_json::json!({ "name": name, "browser_download_url": format!("https://x/{name}") });
        if let Some(d) = digest {
            asset["digest"] = format!("sha256:{d}").into();
        }
        serde_json::json!({ "tag_name": tag, "assets": [{ "name": "other.zip", "browser_download_url": "o", "digest": "sha256:00" }, asset] })
    }

    #[test]
    fn picks_the_platform_file_and_its_digest() {
        let (m, src) = source(|_| {});
        let found = select(&m, &src, &release(&m, "v1.2.3", Some("abcd"))).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].version, "1.2.3");
        assert_eq!(found[0].sha256.as_deref(), Some("abcd"));
        assert!(found[0].url.ends_with(&src.asset().unwrap().file.replace("{version}", "1.2.3")));
    }

    #[test]
    fn newest_skips_pre_releases_and_releases_without_this_platform() {
        let (m, src) = source(|s| s.versions = Versions::Newest);
        let mut rc = release(&m, "v2.0.0-rc1", Some("aa"));
        rc["prerelease"] = true.into();
        let list = serde_json::json!([rc, { "tag_name": "v1.9.0", "assets": [] }, release(&m, "v1.8.0", Some("ff"))]);
        assert_eq!(select(&m, &src, &list).unwrap()[0].version, "1.8.0");
    }

    #[test]
    fn per_major_keeps_the_newest_of_each_major_from_min() {
        let (m, src) = source(|s| {
            s.versions = Versions::PerMajor;
            s.min = Some(14);
            s.label = Some("Echo {major}".into());
        });
        let list = serde_json::json!(["17.2", "17.6", "16.9", "13.20"].map(|v| release(&m, v, Some("ab"))));
        let found = select(&m, &src, &list).unwrap();
        let versions: Vec<&str> = found.iter().map(|f| f.version.as_str()).collect();
        assert_eq!(versions, ["17.6", "16.9"]);
        assert_eq!(found[0].label.as_deref(), Some("Echo 17"));
    }

    #[test]
    fn per_line_keeps_the_newest_lines() {
        let (m, src) = source(|s| {
            s.versions = Versions::PerLine;
            s.keep = Some(2);
        });
        let list = serde_json::json!(["8.2.1", "8.2.3", "8.0.5", "7.4.9", "7.2.1"].map(|v| release(&m, v, Some("ab"))));
        let versions: Vec<String> = select(&m, &src, &list).unwrap().into_iter().map(|f| f.version).collect();
        assert_eq!(versions, ["8.2.3", "8.0.5"]);
    }

    #[test]
    fn tag_suffixes_pick_and_label_releases() {
        let (m, src) = source(|s| {
            s.versions = Versions::Newest;
            s.tags = Ordered(vec![("-lts".into(), "LTS".into()), ("-stable".into(), "Stable".into())]);
            s.assets = Ordered(vec![(platform(), Asset { file: "echo-{version}.tgz".into(), archive: Archive::TarGz, fallbacks: vec![] })]);
        });
        let r = |tag: &str| serde_json::json!({ "tag_name": tag, "prerelease": true, "assets": [{ "name": format!("echo-{}.tgz", &tag[1..tag.rfind('-').unwrap()]), "browser_download_url": "u", "digest": "sha256:ab" }] });
        let found = select(&m, &src, &serde_json::json!([r("v25.9.1-testing"), r("v25.8.2-lts")])).unwrap();
        assert_eq!((found[0].version.as_str(), found[0].label.as_deref()), ("25.8.2", Some("LTS")));
    }

    #[test]
    fn a_vendor_download_needs_its_checksum_file() {
        let (m, src) = source(|s| {
            s.versions = Versions::NewestTag;
            s.url = Some("https://dist.example.org/{file}".into());
            s.checksum = Checksum::Sha512("{url}.sha512".into());
        });
        let tags = serde_json::json!([{ "name": "5.26.0" }, { "name": "5.9.1" }, { "name": "6.0.0-rc1" }, { "name": "5.26.10" }]);
        let found = select(&m, &src, &tags).unwrap();
        assert_eq!(found[0].version, "5.26.10");
        let file = src.asset().unwrap().file.replace("{version}", "5.26.10");
        assert_eq!(found[0].url, format!("https://dist.example.org/{file}"));
        assert_eq!(found[0].checksum_file, Some((Hash::Sha512, format!("https://dist.example.org/{file}.sha512"))));
        // GitHub's digest can't vouch for another host's file.
        let text = SMALL.replace("  assets:\n", "  url: \"https://dist.example.org/{file}\"\n  assets:\n");
        assert!(parse(&text, false).unwrap_err().to_string().contains("needs checksum"));
    }

    #[test]
    fn npm_packages_are_checked_against_their_integrity() {
        let text = SMALL.replace("  github: someone/echo\n", "  npm: echo-server\n  checksum: npm\n  label: \"Node {node}\"\n").replace(
            "  assets:\n    windows: { file: \"echo-{version}-windows.zip\", archive: zip }\n    linux-x64: { file: \"echo-{version}-linux.tar.gz\", archive: targz }\n    linux-arm64: { file: \"echo-{version}-linux-arm.tar.gz\", archive: targz }\n    macos: { file: \"echo-{version}-macos.tar.gz\", archive: targz }\n",
            "",
        );
        let m = parse(&text, false).unwrap();
        let src = m.source.here().unwrap();
        assert_eq!(catalog_url(src).unwrap(), "https://registry.npmjs.org/echo-server/latest");
        // sha512-<base64 of 64 bytes of 0xab>
        let integrity = format!("sha512-{}", { use base64::Engine; base64::engine::general_purpose::STANDARD.encode([0xab; 64]) });
        let latest = serde_json::json!({ "version": "2.1.0", "engines": { "node": ">=20" }, "dist": { "tarball": "https://registry.npmjs.org/t.tgz", "integrity": integrity } });
        let found = select(&m, src, &latest).unwrap();
        assert_eq!(found[0].sha512.as_deref(), Some("ab".repeat(64).as_str()));
        assert_eq!(found[0].label.as_deref(), Some("Node >=20"));
    }

    #[test]
    fn the_first_source_for_this_platform_wins() {
        let text = SMALL.replace("source:\n  github: someone/echo\n", &format!("source:\n  - {{ on: [{}], github: other/one, assets: {{ {}: {{ file: x, archive: file }} }} }}\n  - github: someone/echo\n", platform(), platform()));
        // The second entry keeps the original indentation of the assets block.
        let text = text.replace("\n  assets:\n    windows", "\n    assets:\n      windows").replace("\n    linux-x64", "\n      linux-x64").replace("\n    linux-arm64", "\n      linux-arm64").replace("\n    macos", "\n      macos");
        let m = parse(&text, false).unwrap();
        assert_eq!(m.source.here().unwrap().github.as_deref(), Some("other/one"));
    }

    const OPTIONS: &str = r#"settings:
  memory: { label: Memory, type: number, default: 128, min: 16, max: 1024, unit: MB }
  verbose: { label: Verbose, type: toggle, default: false }
  level: { label: Level, type: choice, options: [info, debug], default: info }
  name: { label: Name, type: text, default: echo }
files:
  - { path: debug.conf, content: "level={{setting.level}}", if: setting.verbose }
"#;

    #[test]
    fn settings_fill_placeholders_and_conditions() {
        let text = format!("{SMALL}{OPTIONS}").replace(
            "{ values: [--unix, \"yes\"], on: [unix] }]",
            "{ values: [--unix, \"yes\"], on: [unix] }, -m, \"{{setting.memory}}\", { value: -v, if: setting.verbose }, { value: --quiet, if: \"!setting.verbose\" }]",
        );
        let m = parse(&text, false).unwrap();
        let mut settings = Settings::default();
        settings.versions.set("echo", Some("1.0.0".into()));
        let layout = Layout::new(std::env::temp_dir().join("lr-manifest-options"));
        let args = |settings: &Settings| Ctx::new(&m, settings, &layout).unwrap().args(&m.run.as_ref().unwrap().args).unwrap();
        let before = args(&settings);
        assert!(before.ends_with(&["-m".into(), "128".into(), "--quiet".into()]), "{before:?}");

        m.set_option(&mut settings, "memory", " 256 ").unwrap();
        m.set_option(&mut settings, "verbose", "true").unwrap();
        let after = args(&settings);
        assert!(after.ends_with(&["-m".into(), "256".into(), "-v".into()]), "{after:?}");
        assert_eq!(settings.service_options["echo"].get("memory").map(String::as_str), Some("256"));

        // The file guarded by the toggle is written only while it's on.
        let ctx = Ctx::new(&m, &settings, &layout).unwrap();
        let _ = std::fs::remove_dir_all(layout.etc("echo"));
        ctx.prepare().unwrap();
        assert_eq!(std::fs::read_to_string(layout.etc("echo").join("debug.conf")).unwrap(), "level=info");

        // Back to the default: nothing is stored.
        m.set_option(&mut settings, "memory", "128").unwrap();
        m.set_option(&mut settings, "verbose", "false").unwrap();
        assert!(!settings.service_options.contains_key("echo"));
    }

    #[test]
    fn setting_values_are_checked() {
        let m = parse(&format!("{SMALL}{OPTIONS}"), false).unwrap();
        let mut settings = Settings::default();
        let bad = |settings: &mut Settings, name: &str, value: &str, says: &str| {
            let e = m.set_option(settings, name, value).unwrap_err().to_string();
            assert!(e.contains(says), "{name}={value}: {e}");
        };
        bad(&mut settings, "memory", "8", "between 16 and 1024");
        bad(&mut settings, "memory", "lots", "whole number");
        bad(&mut settings, "verbose", "yes", "on or off");
        bad(&mut settings, "level", "trace", "one of info, debug");
        bad(&mut settings, "name", "a\nb", "line breaks");
        bad(&mut settings, "nope", "1", "no setting named nope");
        // A stored value the manifest no longer allows falls back to the default.
        settings.service_options.entry("echo".into()).or_default().insert("level".into(), "trace".into());
        assert_eq!(m.setting(&settings, "level").as_deref(), Some("info"));
    }

    #[test]
    fn setting_definitions_are_checked() {
        let bad = |extra: &str, says: &str| {
            let e = with(extra).unwrap_err();
            assert!(e.contains(says), "{extra}: {e}");
        };
        bad("settings:\n  m: { label: M, type: number, default: 5, min: 10 }\n", "the default must be 10 or more");
        bad("settings:\n  c: { label: C, type: choice, default: a }\n", "needs options");
        bad("settings:\n  t: { label: T, type: toggle, default: false, unit: MB }\n", "only for type: number");
        bad("settings:\n  Bad: { label: B, type: text, default: x }\n", "lowercase");
        bad("dirs: [{ value: \"{{data}}/x\", if: setting.nope }]\n", "for a setting in settings");
        bad("config: \"{{setting.nope}}\"\n", "unknown placeholder");
    }

    #[test]
    fn merge_replaces_only_the_keys_it_sets() {
        let shipped = "a=1\n#b=2\nb=3\nc=4\n";
        let out = merge_keys(shipped, &[("b".into(), "9".into())]);
        assert!(out.starts_with("a=1\n#b=2\nc=4\n"));
        assert!(out.ends_with("b=9\n"));
    }
}
