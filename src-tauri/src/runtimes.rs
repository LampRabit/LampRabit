//! Runtime catalog (what can be downloaded) and installer (download, verify, extract).

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use crate::{
    error::{Error, Result},
    layout::{Layout, exe},
};

/// Serialized as its id (`"php"`, or a non-built-in manifest's id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Runtime {
    Apache,
    Nginx,
    Php,
    Mariadb,
    /// Developer tools: a mail catcher and two database web UIs.
    Mailpit,
    Adminer,
    Phpmyadmin,
    /// Public tunnels for sharing sites.
    Ngrok,
    Cloudflared,
    /// Redis with redis-cli: LampRabit's builds on Linux and macOS, the community
    /// redis-windows build (Redis compiled with MSYS2) on Windows.
    Redis,
    /// PHP's package manager, a single `composer.phar` run by the active PHP.
    Composer,
    /// Oracle MySQL, next to MariaDB on its own port.
    Mysql,
    /// Web interface for Redis, served like phpMyAdmin.
    Phpredisadmin,
    /// Memcached: LampRabit's builds on Linux and macOS, nono303's Cygwin build
    /// (repackaged by LampRabit's CI) on Windows.
    Memcached,
    /// Caddy, a single binary from its GitHub releases.
    Caddy,
    /// Java (Eclipse Temurin JDKs from Adoptium), for Java projects and the services that
    /// run on it (Tomcat, Neo4j, OpenSearch).
    Java,
    /// Apache Tomcat, the servlet container for Java web apps; runs on the active Java.
    Tomcat,
    /// FrankenPHP: Caddy with PHP built in, from its GitHub releases.
    Frankenphp,
    /// PostgreSQL, from theseus-rs/postgresql-binaries (built from source for every OS).
    Postgres,
    /// Python, from astral-sh/python-build-standalone (relocatable builds, what uv uses).
    Python,
    /// Web interface for PostgreSQL, served like phpMyAdmin (a maintained fork that runs
    /// on PHP 8; the original stopped at PHP 7).
    Phppgadmin,
    /// pgAdmin 4, PostgreSQL's own admin tool: a Python web app in its own virtual
    /// environment on the active Python.
    Pgadmin,
    /// MongoDB Community Server, from MongoDB's own downloads.
    Mongodb,
    /// MongoDB's shell, from its GitHub releases.
    Mongosh,
    /// MongoDB Database Tools (mongodump, mongorestore…), for backups.
    Mongotools,
    /// MongoDB Compass, MongoDB's desktop app for browsing data.
    Compass,
    /// The sqlite3 command-line shell from sqlite.org (PHP has SQLite built in).
    Sqlite,
    /// ClickHouse, a single binary (Linux and macOS only).
    Clickhouse,
    /// Neo4j Community, a graph database on the active Java.
    Neo4j,
    /// Meilisearch, a search engine (single binary).
    Meilisearch,
    /// Typesense, a search engine (Linux and macOS only).
    Typesense,
    /// RustFS, S3-compatible object storage.
    Rustfs,
    /// Ollama, for running language models locally.
    Ollama,
    /// MinIO's last release, built from source by LampRabit's CI (the project is archived).
    Minio,
    /// Elasticsearch, with its bundled Java.
    Elasticsearch,
    /// OpenSearch, with its bundled Java (no macOS build).
    Opensearch,
    /// n8n, an npm package installed into its own folder and run on NVM's Node.
    N8n,
    /// RabbitMQ, on Erlang.
    Rabbitmq,
    /// Erlang/OTP for RabbitMQ: official builds on Windows only.
    Erlang,
    /// Go, from go.dev.
    Go,
    /// Apache Maven, Java's build tool, on the active Java.
    Maven,
    /// Gradle, Java's other build tool, on the active Java.
    Gradle,
    /// Rust through rustup, with its homes inside LampRabit's folder.
    Rust,
    /// Git for the Git tab and the terminal when the computer has none: MinGit, Git for
    /// Windows' official portable build. Linux and macOS use the system's Git.
    Git,
    /// Bun, a JavaScript runtime and package manager (single binary from its GitHub releases).
    Bun,
    /// Deno, a JavaScript and TypeScript runtime with its own package manager.
    Deno,
    /// uv, Astral's Python package and project manager (single binary).
    Uv,
    /// conda through Miniforge (conda-forge's installer), installed into its own folder.
    Conda,
    /// The runtime of a service that exists only as a manifest: an index into
    /// `manifest::extras()`, the same as its `ServiceKind::Plugin`.
    Plugin(u16),
}

impl Serialize for Runtime {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for Runtime {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let id = String::deserialize(d)?;
        Self::from_id(&id).ok_or_else(|| serde::de::Error::custom(format!("unknown runtime \"{id}\"")))
    }
}

impl Runtime {
    pub const BUILT_IN: [Runtime; 48] = [
        Runtime::Apache,
        Runtime::Nginx,
        Runtime::Php,
        Runtime::Mariadb,
        Runtime::Mailpit,
        Runtime::Adminer,
        Runtime::Phpmyadmin,
        Runtime::Ngrok,
        Runtime::Cloudflared,
        Runtime::Redis,
        Runtime::Composer,
        Runtime::Mysql,
        Runtime::Phpredisadmin,
        Runtime::Memcached,
        Runtime::Caddy,
        Runtime::Java,
        Runtime::Tomcat,
        Runtime::Frankenphp,
        Runtime::Postgres,
        Runtime::Python,
        Runtime::Phppgadmin,
        Runtime::Pgadmin,
        Runtime::Mongodb,
        Runtime::Mongosh,
        Runtime::Mongotools,
        Runtime::Compass,
        Runtime::Sqlite,
        Runtime::Clickhouse,
        Runtime::Neo4j,
        Runtime::Meilisearch,
        Runtime::Typesense,
        Runtime::Rustfs,
        Runtime::Ollama,
        Runtime::Minio,
        Runtime::Elasticsearch,
        Runtime::Opensearch,
        Runtime::N8n,
        Runtime::Rabbitmq,
        Runtime::Erlang,
        Runtime::Go,
        Runtime::Rust,
        Runtime::Maven,
        Runtime::Gradle,
        Runtime::Git,
        Runtime::Bun,
        Runtime::Deno,
        Runtime::Uv,
        Runtime::Conda,
    ];

    /// The built-in runtimes, then those of non-built-in manifests.
    pub fn all() -> Vec<Self> {
        let mut all = Self::BUILT_IN.to_vec();
        all.extend(crate::manifest::extra_indexes().into_iter().map(Self::Plugin));
        all
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::all().into_iter().find(|r| r.id() == id)
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Apache => "apache",
            Self::Nginx => "nginx",
            Self::Php => "php",
            Self::Mariadb => "mariadb",
            Self::Mailpit => "mailpit",
            Self::Adminer => "adminer",
            Self::Phpmyadmin => "phpmyadmin",
            Self::Ngrok => "ngrok",
            Self::Cloudflared => "cloudflared",
            Self::Redis => "redis",
            Self::Composer => "composer",
            Self::Mysql => "mysql",
            Self::Phpredisadmin => "phpredisadmin",
            Self::Memcached => "memcached",
            Self::Caddy => "caddy",
            Self::Java => "java",
            Self::Tomcat => "tomcat",
            Self::Frankenphp => "frankenphp",
            Self::Postgres => "postgres",
            Self::Python => "python",
            Self::Phppgadmin => "phppgadmin",
            Self::Pgadmin => "pgadmin",
            Self::Mongodb => "mongodb",
            Self::Mongosh => "mongosh",
            Self::Mongotools => "mongotools",
            Self::Compass => "compass",
            Self::Sqlite => "sqlite",
            Self::Clickhouse => "clickhouse",
            Self::Neo4j => "neo4j",
            Self::Meilisearch => "meilisearch",
            Self::Typesense => "typesense",
            Self::Rustfs => "rustfs",
            Self::Ollama => "ollama",
            Self::Minio => "minio",
            Self::Elasticsearch => "elasticsearch",
            Self::Opensearch => "opensearch",
            Self::N8n => "n8n",
            Self::Rabbitmq => "rabbitmq",
            Self::Erlang => "erlang",
            Self::Go => "go",
            Self::Rust => "rust",
            Self::Maven => "maven",
            Self::Gradle => "gradle",
            Self::Git => "git",
            Self::Bun => "bun",
            Self::Deno => "deno",
            Self::Uv => "uv",
            Self::Conda => "conda",
            Self::Plugin(i) => &crate::manifest::extra(i).id,
        }
    }

    /// File that must exist inside an installed version's directory.
    pub fn marker(self) -> PathBuf {
        // A service's comes from its manifest's `binary`.
        if let Some(m) = crate::manifest::get(self.id()) {
            return m.marker();
        }
        match self {
            Self::Php if cfg!(windows) => PathBuf::from(exe("php")),
            Self::Php => PathBuf::from("php-fpm"),
            Self::Adminer => PathBuf::from("adminer.php"),
            Self::Phpmyadmin => PathBuf::from("index.php"),
            Self::Ngrok => PathBuf::from(exe("ngrok")),
            Self::Cloudflared => PathBuf::from(exe("cloudflared")),
            Self::Composer => PathBuf::from("composer.phar"),
            Self::Phpredisadmin => PathBuf::from("index.php"),
            Self::Java => Path::new("bin").join(exe("java")),
            Self::Python if cfg!(windows) => PathBuf::from("python.exe"),
            Self::Python => Path::new("bin").join("python3"),
            Self::Phppgadmin => PathBuf::from("index.php"),
            Self::Mongosh => Path::new("bin").join(exe("mongosh")),
            Self::Mongotools => Path::new("bin").join(exe("mongodump")),
            // Compass is an Electron app: its executable's name varies, its app bundle doesn't.
            Self::Compass if cfg!(target_os = "macos") => Path::new("MongoDB Compass.app").join("Contents").join("Resources").join("app.asar"),
            Self::Compass => Path::new("resources").join("app.asar"),
            Self::Sqlite => PathBuf::from(exe("sqlite3")),
            Self::Erlang => Path::new("bin").join(exe("erl")),
            Self::Go => Path::new("bin").join(exe("go")),
            // Written last by `setup_rust`, so a half-finished install doesn't count.
            Self::Rust => PathBuf::from("installed.txt"),
            Self::Maven => Path::new("bin").join("mvn"),
            Self::Gradle => Path::new("bin").join("gradle"),
            Self::Git => Path::new("cmd").join("git.exe"),
            Self::Bun => PathBuf::from(exe("bun")),
            Self::Deno => PathBuf::from(exe("deno")),
            Self::Uv => PathBuf::from(exe("uv")),
            // Written last by `setup_conda`, so a half-finished install doesn't count.
            Self::Conda => PathBuf::from("installed.txt"),
            // Services: from their manifest, above.
            _ => PathBuf::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Archive {
    Zip,
    TarGz,
    TarXz,
    /// Zstandard-compressed tar (Ollama on Linux).
    TarZst,
    /// A single file, saved as the runtime's marker file.
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub runtime: Runtime,
    pub version: String,
    pub url: String,
    pub sha256: Option<String>,
    /// For publishers that only give SHA-512.
    #[serde(default)]
    pub sha512: Option<String>,
    /// MySQL only publishes MD5: it catches corrupt downloads, HTTPS covers the rest.
    #[serde(default)]
    pub md5: Option<String>,
    /// SQLite publishes SHA3-256.
    #[serde(default)]
    pub sha3_256: Option<String>,
    pub archive: Archive,
    /// Short note shown next to the version, e.g. "LTS".
    pub label: Option<String>,
}

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("lampRabit/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .build()
        .expect("http client")
}

fn unsupported(what: &str) -> Error {
    Error::msg(format!("{what} downloads are not available for this OS yet"))
}

fn http(e: reqwest::Error) -> Error {
    Error::msg(format!("network error: {e}"))
}

/// A version's release line: "8.4" for "8.4.27".
fn line(v: &str) -> String {
    v.split('.').take(2).collect::<Vec<_>>().join(".")
}

use crate::version::numbers as version_key;

/// How long a downloaded catalog is used before it's fetched again.
const CATALOG_TTL: Duration = Duration::from_secs(6 * 60 * 60);

fn catalog_file(runtime: Runtime) -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("LampRabit").join("catalogs").join(format!("{}-{}.json", runtime.id(), std::env::consts::OS)))
}

/// The versions available for a runtime. Catalogs are kept on disk for a few hours: most
/// come from GitHub's API, which allows few anonymous requests an hour, and browsing the
/// Services page would otherwise use them up. `fresh` (the refresh button) fetches anew; a
/// failed fetch falls back to the last catalog, however old.
pub async fn available(client: &reqwest::Client, runtime: Runtime, fresh: bool) -> Result<Vec<Release>> {
    let file = catalog_file(runtime);
    let cached = file.as_ref().and_then(|f| {
        let age = std::fs::metadata(f).ok()?.modified().ok()?.elapsed().unwrap_or_default();
        let releases: Vec<Release> = serde_json::from_slice(&std::fs::read(f).ok()?).ok()?;
        Some((age, releases))
    });
    if let Some((age, releases)) = &cached
        && !fresh && *age < CATALOG_TTL && !releases.is_empty() {
            return Ok(releases.clone());
        }
    match fetch_available(client, runtime).await {
        Ok(releases) => {
            if let (Some(f), Ok(json)) = (&file, serde_json::to_vec(&releases)) {
                let _ = std::fs::create_dir_all(f.parent().unwrap_or(f));
                let _ = std::fs::write(f, json);
            }
            Ok(releases)
        }
        Err(e) => match cached {
            Some((_, releases)) if !releases.is_empty() => Ok(releases),
            _ if e.to_string().contains("rate limit") => Err(Error::msg(
                "GitHub's limit for downloads lists was reached (60 an hour without an account). Try again within the hour, or set a GITHUB_TOKEN environment variable and restart LampRabit.",
            )),
            _ => Err(e),
        },
    }
}

async fn fetch_available(client: &reqwest::Client, runtime: Runtime) -> Result<Vec<Release>> {
    let mut releases = match runtime {
        // Services whose manifest says where their releases are (src-tauri/services/).
        _ if crate::manifest::get(runtime.id()).is_some() => manifest_releases(client, runtime).await?,
        Runtime::Php => php_releases(client).await?,
        Runtime::Adminer => adminer_releases(client).await?,
        Runtime::Phpmyadmin => phpmyadmin_releases(client).await?,
        Runtime::Ngrok => ngrok_releases(),
        Runtime::Cloudflared => cloudflared_releases(client).await?,
        Runtime::Composer => composer_releases(client).await?,
        Runtime::Phpredisadmin => phpredisadmin_releases(client).await?,
        Runtime::Java => java_releases(client).await?,
        Runtime::Python => python_releases(client).await?,
        Runtime::Phppgadmin => phppgadmin_releases(client).await?,
        Runtime::Mongosh => mongosh_releases(client).await?,
        Runtime::Mongotools => mongotools_releases(client).await?,
        Runtime::Compass => compass_releases(client).await?,
        Runtime::Sqlite => sqlite_releases(client).await?,
        Runtime::Erlang => erlang_releases(client).await?,
        Runtime::Go => go_releases(client).await?,
        Runtime::Rust => rust_releases(client).await?,
        Runtime::Maven => maven_releases(client).await?,
        Runtime::Gradle => gradle_releases(client).await?,
        Runtime::Git => git_releases(client).await?,
        Runtime::Bun => bun_releases(client).await?,
        Runtime::Deno => deno_releases(client).await?,
        Runtime::Uv => uv_releases(client).await?,
        Runtime::Conda => conda_releases(client).await?,
        _ => return Err(unsupported(runtime.id())),
    };
    releases.sort_by_key(|r| std::cmp::Reverse(version_key(&r.version)));
    Ok(releases)
}

/// Where LampRabit's own builds are published: Apache, Nginx, PHP, Redis and Memcached for
/// Linux and macOS, and Memcached for Windows (see build/linux/ and
/// .github/workflows/binaries.yml).
/// `LAMPRABIT_BINARIES_URL` points elsewhere, for testing new builds or a mirror.
const LINUX_BUILDS: &str = "https://github.com/LampRabit/LampRabit/releases/download/binaries/";

/// LampRabit's builds of one runtime for this OS and CPU, from the published manifest.
async fn own_builds(client: &reqwest::Client, runtime: Runtime) -> Result<Vec<Release>> {
    let base = std::env::var("LAMPRABIT_BINARIES_URL").unwrap_or_else(|_| LINUX_BUILDS.to_string());
    let base = format!("{}/", base.trim_end_matches('/'));
    let manifest: Value = client
        .get(format!("{base}manifest.json"))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| Error::msg(format!("Couldn't load the list of LampRabit's builds: {e}")))?
        .json()
        .await
        .map_err(http)?;
    let releases: Vec<Release> = manifest["releases"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|e| e["runtime"] == runtime.id() && e["os"] == std::env::consts::OS && e["arch"] == std::env::consts::ARCH)
        .filter_map(|e| {
            Some(Release {
                runtime,
                version: e["version"].as_str()?.to_string(),
                url: format!("{base}{}", e["file"].as_str()?),
                sha256: Some(e["sha256"].as_str()?.to_string()),
                sha512: None,
                md5: None,
                sha3_256: None,
                archive: Archive::TarGz,
                label: e["label"].as_str().map(str::to_string),
            })
        })
        .collect();
    if releases.is_empty() {
        return Err(Error::msg(format!(
            "There are no {} builds of {} for {} yet",
            std::env::consts::OS,
            runtime.id(),
            std::env::consts::ARCH
        )));
    }
    // Only the newest patch release of each line, like the Windows catalogs.
    let mut newest: Vec<Release> = Vec::new();
    for r in releases {
        match newest.iter_mut().find(|n| line(&n.version) == line(&r.version)) {
            Some(n) if version_key(&r.version) > version_key(&n.version) => *n = r,
            Some(_) => {}
            None => newest.push(r),
        }
    }
    Ok(newest)
}

/// windows.php.net lists the newest patch release of every branch, with checksums.
async fn php_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    if !cfg!(windows) {
        return own_builds(client, Runtime::Php).await;
    }
    const BASE: &str = "https://windows.php.net/downloads/releases/";
    let json: Value = client
        .get(format!("{BASE}releases.json"))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(http)?
        .json()
        .await
        .map_err(http)?;

    let mut out = Vec::new();
    for branch in json.as_object().into_iter().flat_map(|o| o.values()) {
        let Some(version) = branch["version"].as_str() else { continue };
        // Older branches are end-of-life and have known incompatibilities with current Apache builds.
        if version_key(version) < vec![8, 1] {
            continue;
        }
        // Thread-safe x64 build, required by mod_php.
        let Some(build) = branch
            .as_object()
            .and_then(|o| o.iter().find(|(k, _)| k.starts_with("ts-") && k.ends_with("-x64")))
            .map(|(_, v)| v)
        else {
            continue;
        };
        let Some(path) = build["zip"]["path"].as_str() else { continue };
        out.push(Release {
            runtime: Runtime::Php,
            version: version.to_string(),
            url: format!("{BASE}{path}"),
            sha256: build["zip"]["sha256"].as_str().map(str::to_string),
            sha512: None,
            md5: None,
            sha3_256: None,
            archive: Archive::Zip,
            // "8.4.27-1": the same PHP release rebuilt by windows.php.net (newer libraries).
            label: version.split_once('-').map(|(base, _)| format!("Rebuild of {base}")),
        });
    }
    Ok(out)
}

async fn get_json(client: &reqwest::Client, url: &str) -> Result<Value> {
    let mut request = client.get(url);
    // GitHub's API wants its own media type (other hosts, npm's registry for one, refuse
    // it), and allows 60 anonymous requests an hour; a token raises that to 5000.
    if url.starts_with("https://api.github.com/") {
        request = request.header("Accept", "application/vnd.github+json");
        if let Some(token) = ["GITHUB_TOKEN", "GH_TOKEN"].iter().find_map(|v| std::env::var(v).ok().filter(|t| !t.is_empty())) {
            request = request.bearer_auth(token);
        }
    }
    request
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(http)?
        .json()
        .await
        .map_err(http)
}

/// Latest GitHub release of `repo`, picking the asset `pick` accepts. GitHub reports a
/// sha256 digest for each asset.
async fn github_latest(
    client: &reqwest::Client,
    runtime: Runtime,
    repo: &str,
    archive: Archive,
    pick: impl Fn(&str, &str) -> bool,
) -> Result<Vec<Release>> {
    let latest = get_json(client, &format!("https://api.github.com/repos/{repo}/releases/latest")).await?;
    let version = latest["tag_name"].as_str().unwrap_or_default().trim_start_matches('v').to_string();
    let asset = latest["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| a["name"].as_str().is_some_and(|n| pick(n, &version)))
        .ok_or_else(|| Error::msg(format!("no download for this platform in {repo} {version}")))?;
    Ok(vec![Release {
        runtime,
        url: asset["browser_download_url"].as_str().unwrap_or_default().to_string(),
        sha256: asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).map(str::to_string),
        sha512: None,
        md5: None,
        sha3_256: None,
        version,
        archive,
        label: None,
    }])
}


/// The newest Temurin JDK of every LTS line, plus the newest release when it isn't one.
/// Versions drop the build number ("21.0.5", not "21.0.5+11"), so folders and shims
/// read like the other runtimes.
async fn java_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let os = match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "mac",
        "linux" => "linux",
        _ => return Err(unsupported("Java")),
    };
    let arch = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x64" };
    let archive = if cfg!(windows) { Archive::Zip } else { Archive::TarGz };
    let info = get_json(client, "https://api.adoptium.net/v3/info/available_releases").await?;
    let lts: Vec<u64> = info["available_lts_releases"].as_array().into_iter().flatten().filter_map(Value::as_u64).collect();
    let mut lines: Vec<u64> = lts.iter().copied().filter(|&v| v >= 8).collect();
    if let Some(newest) = info["most_recent_feature_release"].as_u64().filter(|v| !lts.contains(v)) {
        lines.push(newest);
    }
    let lts = &lts;
    let lookups = lines.into_iter().map(|line| async move {
        let url = format!("https://api.adoptium.net/v3/assets/latest/{line}/hotspot?architecture={arch}&image_type=jdk&os={os}&vendor=eclipse");
        let assets = get_json(client, &url).await.ok()?;
        let asset = assets.as_array()?.first()?;
        let package = &asset["binary"]["package"];
        let semver = asset["version"]["semver"].as_str()?;
        Some(Release {
            runtime: Runtime::Java,
            version: semver.split('+').next().unwrap_or(semver).to_string(),
            url: package["link"].as_str()?.to_string(),
            sha256: package["checksum"].as_str().map(str::to_string),
            sha512: None,
            md5: None,
            sha3_256: None,
            archive,
            label: Some(if lts.contains(&line) { "LTS".into() } else { "Latest".into() }),
        })
    });
    let releases: Vec<Release> = futures_util::future::join_all(lookups).await.into_iter().flatten().collect();
    if releases.is_empty() {
        return Err(Error::msg("Couldn't load the Java versions from Adoptium. Check your connection and try again."));
    }
    Ok(releases)
}

/// The newest patch of every stable Python line from the latest python-build-standalone
/// release (its "install_only" archives: a plain Python folder, no debug files).
async fn python_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let arm = cfg!(target_arch = "aarch64");
    let target = match (std::env::consts::OS, arm) {
        ("windows", false) => "x86_64-pc-windows-msvc",
        ("windows", true) => "aarch64-pc-windows-msvc",
        ("linux", false) => "x86_64-unknown-linux-gnu",
        ("linux", true) => "aarch64-unknown-linux-gnu",
        ("macos", false) => "x86_64-apple-darwin",
        ("macos", true) => "aarch64-apple-darwin",
        _ => return Err(unsupported("Python")),
    };
    let latest = get_json(client, "https://api.github.com/repos/astral-sh/python-build-standalone/releases/latest").await?;
    let suffix = format!("-{target}-install_only.tar.gz");
    let mut newest: std::collections::BTreeMap<String, Release> = Default::default();
    for asset in latest["assets"].as_array().into_iter().flatten() {
        let name = asset["name"].as_str().unwrap_or_default();
        // cpython-3.13.18+20261026-x86_64-pc-windows-msvc-install_only.tar.gz
        let Some(version) = name.strip_prefix("cpython-").and_then(|n| n.strip_suffix(&suffix)).and_then(|n| n.split('+').next()) else { continue };
        // Stable releases only (no 3.16.0a1).
        if !version.split('.').all(|p| p.parse::<u32>().is_ok()) {
            continue;
        }
        let line = line(version);
        if newest.get(&line).is_some_and(|r| version_key(&r.version) >= version_key(version)) {
            continue;
        }
        newest.insert(line, Release {
            runtime: Runtime::Python,
            version: version.to_string(),
            url: asset["browser_download_url"].as_str().unwrap_or_default().to_string(),
            sha256: asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).map(str::to_string),
            sha512: None,
            md5: None,
            sha3_256: None,
            archive: Archive::TarGz,
            label: None,
        });
    }
    if newest.is_empty() {
        return Err(Error::msg("No Python builds for this platform were found. Check your connection and try again."));
    }
    Ok(newest.into_values().collect())
}

/// phpPgAdmin from ReimuHakurei's fork, which supports PHP 8 and current PostgreSQL.
/// Its releases are tags without uploaded files, so the download is GitHub's source zip,
/// checked by HTTPS only.
async fn phppgadmin_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let latest = get_json(client, "https://api.github.com/repos/ReimuHakurei/phpPgAdmin/releases/latest").await?;
    let tag = latest["tag_name"].as_str().ok_or_else(|| Error::msg("Couldn't read phpPgAdmin's latest release"))?;
    let version = tag.trim_start_matches('v').trim_end_matches("-mod").to_string();
    Ok(vec![Release {
        runtime: Runtime::Phppgadmin,
        version,
        url: format!("https://github.com/ReimuHakurei/phpPgAdmin/archive/refs/tags/{tag}.zip"),
        sha256: None,
        sha512: None,
        md5: None,
        sha3_256: None,
        archive: Archive::Zip,
        label: Some("PHP 8 fork".into()),
    }])
}

/// Runs a setup command, failing with the last line it printed to stderr.
async fn run_quiet(program: &Path, args: &[String], cwd: &Path) -> Result<()> {
    run_quiet_env(program, args, cwd, &[]).await
}

async fn mongosh_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "x64" };
    let (platform, ext, archive) = match std::env::consts::OS {
        "windows" => ("win32", "zip", Archive::Zip),
        "macos" => ("darwin", "zip", Archive::Zip),
        "linux" => ("linux", "tgz", Archive::TarGz),
        _ => return Err(unsupported("mongosh")),
    };
    github_latest(client, Runtime::Mongosh, "mongodb-js/mongosh", archive, |n, v| n == format!("mongosh-{v}-{platform}-{arch}.{ext}")).await
}

/// The newest MongoDB Database Tools. They're built against old distributions (Ubuntu
/// 18.04, RHEL 8), which also run on newer ones.
async fn mongotools_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let arm = cfg!(target_arch = "aarch64");
    let (name, arch) = match std::env::consts::OS {
        "windows" if !arm => ("windows", "x86_64"),
        "macos" => ("macos", "x86_64"),
        "linux" if arm => ("ubuntu1804", "arm64"),
        "linux" => ("ubuntu1804", "x86_64"),
        _ => return Err(unsupported("MongoDB Database Tools")),
    };
    let full = get_json(client, "https://downloads.mongodb.org/tools/db/full.json").await?;
    let newest = full["versions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|v| v["version"].as_str().is_some_and(|s| !s.contains('-')))
        .max_by_key(|v| version_key(v["version"].as_str().unwrap_or_default()))
        .ok_or_else(|| Error::msg("Couldn't read the MongoDB Database Tools releases"))?;
    let download = newest["downloads"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|d| d["name"].as_str() == Some(name) && d["arch"].as_str() == Some(arch))
        .ok_or_else(|| unsupported("MongoDB Database Tools"))?;
    let url = download["archive"]["url"].as_str().unwrap_or_default().to_string();
    Ok(vec![Release {
        runtime: Runtime::Mongotools,
        version: newest["version"].as_str().unwrap_or_default().to_string(),
        archive: if url.ends_with(".zip") { Archive::Zip } else { Archive::TarGz },
        sha256: download["archive"]["sha256"].as_str().map(str::to_string),
        sha512: None,
        md5: None,
        sha3_256: None,
        url,
        label: None,
    }])
}

async fn compass_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "x64" };
    let (name, archive) = match std::env::consts::OS {
        "windows" => ("win32-x64.zip".to_string(), Archive::Zip),
        "macos" => (format!("darwin-{arch}.zip"), Archive::Zip),
        "linux" => ("linux-x64.tar.gz".to_string(), Archive::TarGz),
        _ => return Err(unsupported("MongoDB Compass")),
    };
    github_latest(client, Runtime::Compass, "mongodb-js/compass", archive, |n, v| n == format!("mongodb-compass-{v}-{name}")).await
}

/// The sqlite3 shell from sqlite.org's download page, whose comment block lists every file
/// with its SHA3-256.
async fn sqlite_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let platform = match (std::env::consts::OS, cfg!(target_arch = "aarch64")) {
        ("windows", false) => "win-x64",
        ("windows", true) => "win-arm64",
        ("linux", false) => "linux-x64",
        ("macos", false) => "osx-x64",
        ("macos", true) => "osx-arm64",
        _ => return Err(unsupported("SQLite")),
    };
    let page = client.get("https://www.sqlite.org/download.html").send().await.and_then(|r| r.error_for_status()).map_err(http)?.text().await.map_err(http)?;
    // PRODUCT,3.54.1,2026/sqlite-tools-win-x64-3540100.zip,6984432,<sha3>
    let row = page
        .lines()
        .filter_map(|l| l.strip_prefix("PRODUCT,"))
        .map(|l| l.split(',').collect::<Vec<_>>())
        .find(|f| f.len() == 4 && f[1].contains(&format!("sqlite-tools-{platform}-")))
        .ok_or_else(|| unsupported("SQLite"))?;
    Ok(vec![Release {
        runtime: Runtime::Sqlite,
        version: row[0].to_string(),
        url: format!("https://www.sqlite.org/{}", row[1]),
        sha256: None,
        sha512: None,
        md5: None,
        sha3_256: Some(row[3].trim().to_string()),
        archive: Archive::Zip,
        label: None,
    }])
}

/// Like `github_latest`, but the newest of the recent releases that has the file: a release
/// sometimes ships without one platform's build (Meilisearch 1.56.2 had no Apple silicon file).
async fn github_newest_with(client: &reqwest::Client, runtime: Runtime, repo: &str, archive: Archive, name: &str) -> Result<Vec<Release>> {
    let list = get_json(client, &format!("https://api.github.com/repos/{repo}/releases?per_page=10")).await?;
    for release in list.as_array().into_iter().flatten().filter(|r| r["prerelease"] == false && r["draft"] == false) {
        let Some(asset) = release["assets"].as_array().into_iter().flatten().find(|a| a["name"] == name) else { continue };
        return Ok(vec![Release {
            runtime,
            version: release["tag_name"].as_str().unwrap_or_default().trim_start_matches('v').to_string(),
            url: asset["browser_download_url"].as_str().unwrap_or_default().to_string(),
            sha256: asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).map(str::to_string),
            sha512: None,
            md5: None,
            sha3_256: None,
            archive,
            label: None,
        }]);
    }
    Err(Error::msg(format!("No recent {repo} release has a download for this platform ({name}).")))
}

/// A service's releases from its manifest's source for this platform (GitHub releases or
/// tags, npm, LampRabit's own builds), with published checksum files fetched. A release
/// whose checksum can't be had is left out: nothing is installed unchecked.
async fn manifest_releases(client: &reqwest::Client, runtime: Runtime) -> Result<Vec<Release>> {
    use crate::manifest;
    let m = manifest::get(runtime.id()).ok_or_else(|| unsupported(runtime.id()))?;
    if let Some(why) = m.unavailable_here() {
        return Err(Error::msg(why.to_string()));
    }
    let src = m.source.here().ok_or_else(|| manifest::no_download(m))?;
    if src.lamprabit {
        return own_builds(client, runtime).await;
    }
    let found = if src.json.is_some() || src.page.is_some() {
        crate::sources::releases(client, m, src).await?
    } else {
        let url = manifest::catalog_url(src).ok_or_else(|| manifest::no_download(m))?;
        manifest::select(m, src, &get_json(client, &url).await?)?
    };
    let mut out: Vec<Release> = Vec::new();
    let mut missing = None;
    for found in found {
        // Entries for a version already found are its fallback files.
        if out.iter().any(|r| r.version == found.version) {
            continue;
        }
        let (mut sha256, mut sha512, mut md5) = (found.sha256, found.sha512, found.md5);
        if let Some((kind, file)) = &found.checksum_file {
            let Some(hash) = published_hash(client, file, *kind).await else {
                missing = Some(file.clone());
                continue;
            };
            match kind {
                manifest::Hash::Sha256 => sha256 = Some(hash),
                manifest::Hash::Sha512 => sha512 = Some(hash),
                manifest::Hash::Md5 => md5 = Some(hash),
            }
        }
        out.push(Release { runtime, version: found.version, url: found.url, sha256, sha512, md5, sha3_256: None, archive: found.archive, label: found.label });
    }
    match (out.is_empty(), missing) {
        (true, Some(file)) => Err(Error::msg(format!("Couldn't get {}'s published checksum ({file}), so LampRabit won't install it. Try again later.", m.name))),
        _ => Ok(out),
    }
}

/// A checksum file published next to an archive: "<hash>  <file>", a bare hash, or a
/// listing with the hash on a line of its own (Apache Lounge's .txt).
async fn published_hash(client: &reqwest::Client, url: &str, kind: crate::manifest::Hash) -> Option<String> {
    let r = client.get(url).send().await.ok()?.error_for_status().ok()?;
    crate::sources::find_hash(&r.text().await.ok()?, kind)
}

/// Erlang/OTP for RabbitMQ: the newest of the two latest major versions, from the
/// official Windows zips. Linux and macOS have no official builds.
async fn erlang_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    if !cfg!(windows) {
        return Err(Error::msg(
            "Erlang has no official Linux or macOS builds. Install it with your system's packages (sudo apt install erlang-nox, sudo dnf install erlang, or brew install erlang); LampRabit's RabbitMQ uses it.",
        ));
    }
    let list = get_json(client, "https://api.github.com/repos/erlang/otp/releases?per_page=40").await?;
    let mut newest: std::collections::BTreeMap<u32, Release> = Default::default();
    for r in list.as_array().into_iter().flatten().filter(|r| !r["prerelease"].as_bool().unwrap_or(false)) {
        let version = r["tag_name"].as_str().unwrap_or_default().trim_start_matches("OTP-").to_string();
        let Some(major) = version.split('.').next().and_then(|m| m.parse::<u32>().ok()) else { continue };
        if newest.get(&major).is_some_and(|n| version_key(&n.version) >= version_key(&version)) {
            continue;
        }
        let name = format!("otp_win64_{version}.zip");
        let Some(asset) = r["assets"].as_array().into_iter().flatten().find(|a| a["name"].as_str() == Some(name.as_str())) else { continue };
        newest.insert(major, Release {
            runtime: Runtime::Erlang,
            url: asset["browser_download_url"].as_str().unwrap_or_default().to_string(),
            sha256: asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).map(str::to_string),
            version,
            sha512: None,
            md5: None,
            sha3_256: None,
            archive: Archive::Zip,
            label: Some(format!("OTP {major}")),
        });
    }
    let mut releases: Vec<Release> = newest.into_values().rev().take(2).collect();
    releases.reverse();
    if releases.is_empty() {
        return Err(Error::msg("Couldn't find Erlang's Windows builds. Check your connection and try again."));
    }
    Ok(releases)
}

/// The newest Maven 4 and Maven 3 from Apache's mirror, SHA-512 checked like Tomcat. The
/// tarball keeps the Unix scripts executable; Windows gets the zip.
async fn maven_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    const MIRROR: &str = "https://dlcdn.apache.org/maven";
    let (ext, archive) = if cfg!(windows) { ("zip", Archive::Zip) } else { ("tar.gz", Archive::TarGz) };
    let lookups = [(4, "Java 17+"), (3, "Java 8+")].map(|(line, label)| async move {
        let listing = client.get(format!("{MIRROR}/maven-{line}/")).send().await.ok()?.error_for_status().ok()?.text().await.ok()?;
        // Only final releases: 4.0.0-rc-4 and the like are skipped.
        let version = listing
            .split("href=\"")
            .skip(1)
            .filter_map(|s| s.split('/').next())
            .filter(|v| v.starts_with(&format!("{line}.")) && v.split('.').all(|p| p.parse::<u32>().is_ok()))
            .max_by_key(|v| version_key(v))?
            .to_string();
        let url = format!("{MIRROR}/maven-{line}/{version}/binaries/apache-maven-{version}-bin.{ext}");
        let sha512 = client.get(format!("{url}.sha512")).send().await.ok()?.error_for_status().ok()?.text().await.ok()?;
        let sha512 = sha512.split_whitespace().next()?.to_ascii_lowercase();
        Some(Release { runtime: Runtime::Maven, version, url, sha256: None, sha512: Some(sha512), md5: None, sha3_256: None, archive, label: Some(label.into()) })
    });
    let releases: Vec<Release> = futures_util::future::join_all(lookups).await.into_iter().flatten().collect();
    if releases.is_empty() {
        return Err(Error::msg("Couldn't load the Maven versions from dlcdn.apache.org. Check your connection and try again."));
    }
    Ok(releases)
}

/// The newest final release of Gradle's two latest major versions, from services.gradle.org,
/// with the SHA-256 Gradle publishes for each distribution (one zip for every OS).
async fn gradle_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let list = get_json(client, "https://services.gradle.org/versions/all").await?;
    let mut newest: std::collections::BTreeMap<u32, (String, String, String)> = Default::default();
    for v in list.as_array().into_iter().flatten() {
        let final_release = !v["snapshot"].as_bool().unwrap_or(true)
            && !v["nightly"].as_bool().unwrap_or(true)
            && !v["broken"].as_bool().unwrap_or(true)
            && v["rcFor"].as_str().unwrap_or_default().is_empty()
            && v["milestoneFor"].as_str().unwrap_or_default().is_empty();
        let (Some(version), Some(url), Some(sum)) = (v["version"].as_str(), v["downloadUrl"].as_str(), v["checksumUrl"].as_str()) else { continue };
        let Some(major) = version.split('.').next().and_then(|m| m.parse::<u32>().ok()) else { continue };
        if final_release && newest.get(&major).is_none_or(|(n, _, _)| version_key(n) < version_key(version)) {
            newest.insert(major, (version.to_string(), url.to_string(), sum.to_string()));
        }
    }
    let mut releases = Vec::new();
    for (version, url, sum_url) in newest.into_values().rev().take(2) {
        let sha256 = client.get(&sum_url).send().await.ok().and_then(|r| r.error_for_status().ok());
        let Some(sha256) = (match sha256 {
            Some(r) => r.text().await.ok().and_then(|t| t.split_whitespace().next().map(str::to_ascii_lowercase)),
            None => None,
        }) else {
            continue;
        };
        releases.push(Release { runtime: Runtime::Gradle, version, url, sha256: Some(sha256), sha512: None, md5: None, sha3_256: None, archive: Archive::Zip, label: None });
    }
    if releases.is_empty() {
        return Err(Error::msg("Couldn't load the Gradle versions from services.gradle.org. Check your connection and try again."));
    }
    Ok(releases)
}

/// The supported Go releases (the newest two lines) from go.dev, with their SHA-256s.
async fn go_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let os = match std::env::consts::OS {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "darwin",
        _ => return Err(unsupported("Go")),
    };
    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "amd64" };
    let list = get_json(client, "https://go.dev/dl/?mode=json").await?;
    let releases: Vec<Release> = list
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["stable"].as_bool() == Some(true))
        .filter_map(|r| {
            let file = r["files"].as_array()?.iter().find(|f| f["os"] == os && f["arch"] == arch && f["kind"] == "archive")?;
            let name = file["filename"].as_str()?;
            Some(Release {
                runtime: Runtime::Go,
                version: r["version"].as_str()?.trim_start_matches("go").to_string(),
                url: format!("https://go.dev/dl/{name}"),
                sha256: file["sha256"].as_str().map(str::to_string),
                sha512: None,
                md5: None,
                sha3_256: None,
                archive: if name.ends_with(".zip") { Archive::Zip } else { Archive::TarGz },
                label: None,
            })
        })
        .collect();
    if releases.is_empty() {
        return Err(unsupported("Go"));
    }
    Ok(releases)
}

/// The host rustup installs for: on Windows the GNU toolchain, which links with its own
/// MinGW tools instead of needing Visual Studio's C++ Build Tools.
fn rust_host() -> Option<&'static str> {
    Some(match (std::env::consts::OS, cfg!(target_arch = "aarch64")) {
        ("windows", false) => "x86_64-pc-windows-gnu",
        ("windows", true) => "aarch64-pc-windows-gnullvm",
        ("linux", false) => "x86_64-unknown-linux-gnu",
        ("linux", true) => "aarch64-unknown-linux-gnu",
        ("macos", false) => "x86_64-apple-darwin",
        ("macos", true) => "aarch64-apple-darwin",
        _ => return None,
    })
}

/// Rust stable, installed by rustup-init (downloaded with its published SHA-256).
async fn rust_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    // rustup-init itself is the host's normal build; on Windows that's the MSVC one.
    let init_target = match (std::env::consts::OS, cfg!(target_arch = "aarch64")) {
        ("windows", false) => "x86_64-pc-windows-msvc",
        ("windows", true) => "aarch64-pc-windows-msvc",
        _ => rust_host().ok_or_else(|| unsupported("Rust"))?,
    };
    let channel = client
        .get("https://static.rust-lang.org/dist/channel-rust-stable.toml")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(http)?
        .text()
        .await
        .map_err(http)?;
    // [pkg.rust]
    // version = "1.99.1 (58cf8f9c3 2026-10-13)"
    let version = channel
        .split("[pkg.rust]")
        .nth(1)
        .and_then(|s| s.lines().find_map(|l| l.strip_prefix("version = \"")))
        .and_then(|v| v.split_whitespace().next())
        .ok_or_else(|| Error::msg("Couldn't read Rust's stable version"))?
        .to_string();
    let url = format!("https://static.rust-lang.org/rustup/dist/{init_target}/rustup-init{}", if cfg!(windows) { ".exe" } else { "" });
    let sha256 = published_hash(client, &format!("{url}.sha256"), crate::manifest::Hash::Sha256).await;
    Ok(vec![Release { runtime: Runtime::Rust, version, url, sha256, sha512: None, md5: None, sha3_256: None, archive: Archive::File, label: Some("Stable".into()) }])
}

/// RUSTUP_HOME and CARGO_HOME for a LampRabit Rust: inside its own folder.
pub fn rust_env(layout: &Layout, version: &str) -> Vec<(String, String)> {
    let home = layout.bin("rust", version);
    vec![
        ("RUSTUP_HOME".into(), home.join("rustup").to_string_lossy().into_owned()),
        ("CARGO_HOME".into(), home.join("cargo").to_string_lossy().into_owned()),
    ]
}

/// Runs the downloaded rustup-init for the given toolchain, without touching PATH or the
/// user's own ~/.cargo and ~/.rustup. Leaves nothing behind when it fails.
pub async fn setup_rust(layout: &Layout, version: &str) -> Result<()> {
    let home = layout.bin("rust", version);
    let result = async {
        let init = home.join(exe("rustup-init"));
        std::fs::rename(home.join("installed.txt"), &init)?;
        let mut args: Vec<String> = ["-y", "--no-modify-path", "--profile", "minimal", "--default-toolchain", version].iter().map(|s| s.to_string()).collect();
        if let Some(host) = rust_host() {
            args.extend(["--default-host".to_string(), host.to_string()]);
        }
        run_quiet_env(&init, &args, &home, &rust_env(layout, version)).await?;
        let _ = std::fs::remove_file(&init);
        std::fs::write(home.join("installed.txt"), format!("Rust {version}\n"))?;
        Ok::<_, Error>(())
    }
    .await;
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&home);
        return Err(Error::msg(format!("Installing Rust failed: {e}")));
    }
    Ok(())
}

async fn run_quiet_env(program: &Path, args: &[String], cwd: &Path, env: &[(String, String)]) -> Result<()> {
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args).current_dir(cwd).stdin(std::process::Stdio::null()).envs(env.iter().map(|(k, v)| (k, v)));
    crate::procgroup::prepare(&mut cmd);
    let out = cmd.output().await?;
    if out.status.success() {
        return Ok(());
    }
    let last = crate::error::last_line(&out.stderr).unwrap_or_default();
    Err(Error::msg(format!("{} failed: {last}", program.display())))
}

/// NVM's Node for n8n: its default version, which must meet n8n's minimum major.
pub fn n8n_node(minimum: u32) -> Result<PathBuf> {
    let (version, bin) = crate::node::for_project(None, Path::new("."))
        .map_err(|_| Error::msg("n8n runs on Node.js: install Node.js 24 or newer on the Services page (Languages → Node.js) first."))?;
    let major: u32 = version.trim_start_matches('v').split('.').next().and_then(|m| m.parse().ok()).unwrap_or(0);
    if major < minimum {
        return Err(Error::msg(format!("n8n needs Node.js {minimum} or newer, and NVM's default is {version}. Install a newer Node.js on the Services page and make it the default.")));
    }
    Ok(bin)
}

/// Bun from its GitHub releases ("bun-v1.3.2" tags). Windows on Arm runs the x64 build.
async fn bun_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let name = match (std::env::consts::OS, cfg!(target_arch = "aarch64")) {
        ("windows", _) => "bun-windows-x64.zip",
        ("linux", false) => "bun-linux-x64.zip",
        ("linux", true) => "bun-linux-aarch64.zip",
        ("macos", false) => "bun-darwin-x64.zip",
        ("macos", true) => "bun-darwin-aarch64.zip",
        _ => return Err(unsupported("Bun")),
    };
    let mut releases = github_newest_with(client, Runtime::Bun, "oven-sh/bun", Archive::Zip, name).await?;
    for r in &mut releases {
        r.version = r.version.trim_start_matches("bun-v").to_string();
    }
    Ok(releases)
}

/// Deno from its GitHub releases. Windows on Arm runs the x64 build.
async fn deno_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let target = match (std::env::consts::OS, cfg!(target_arch = "aarch64")) {
        ("windows", _) => "x86_64-pc-windows-msvc",
        ("linux", false) => "x86_64-unknown-linux-gnu",
        ("linux", true) => "aarch64-unknown-linux-gnu",
        ("macos", false) => "x86_64-apple-darwin",
        ("macos", true) => "aarch64-apple-darwin",
        _ => return Err(unsupported("Deno")),
    };
    github_newest_with(client, Runtime::Deno, "denoland/deno", Archive::Zip, &format!("deno-{target}.zip")).await
}

/// uv from Astral's GitHub releases.
async fn uv_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let (target, archive) = match (std::env::consts::OS, cfg!(target_arch = "aarch64")) {
        ("windows", false) => ("x86_64-pc-windows-msvc.zip", Archive::Zip),
        ("windows", true) => ("aarch64-pc-windows-msvc.zip", Archive::Zip),
        ("linux", false) => ("x86_64-unknown-linux-gnu.tar.gz", Archive::TarGz),
        ("linux", true) => ("aarch64-unknown-linux-gnu.tar.gz", Archive::TarGz),
        ("macos", false) => ("x86_64-apple-darwin.tar.gz", Archive::TarGz),
        ("macos", true) => ("aarch64-apple-darwin.tar.gz", Archive::TarGz),
        _ => return Err(unsupported("uv")),
    };
    github_newest_with(client, Runtime::Uv, "astral-sh/uv", archive, &format!("uv-{target}")).await
}

/// Where uv keeps the Pythons it downloads: inside LampRabit's folder (bin/ is removed on
/// uninstall and moved with the folder), not the user's ~/.local/share/uv.
pub fn uv_env(layout: &Layout) -> Vec<(String, String)> {
    vec![("UV_PYTHON_INSTALL_DIR".into(), layout.root().join("bin").join("uv-python").to_string_lossy().into_owned())]
}

/// Miniforge, conda-forge's conda installer, from its GitHub releases. The installer is
/// saved as the marker and `setup_conda` runs it.
async fn conda_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let name = match (std::env::consts::OS, cfg!(target_arch = "aarch64")) {
        ("windows", _) => "Miniforge3-Windows-x86_64.exe",
        ("linux", false) => "Miniforge3-Linux-x86_64.sh",
        ("linux", true) => "Miniforge3-Linux-aarch64.sh",
        ("macos", false) => "Miniforge3-MacOSX-x86_64.sh",
        ("macos", true) => "Miniforge3-MacOSX-arm64.sh",
        _ => return Err(unsupported("Miniforge")),
    };
    github_newest_with(client, Runtime::Conda, "conda-forge/miniforge", Archive::File, name).await
}

/// The conda program of an installed Miniforge.
pub fn conda_program(home: &Path) -> PathBuf {
    if cfg!(windows) { home.join("conda").join("Scripts").join("conda.exe") } else { home.join("conda").join("bin").join("conda") }
}

/// Runs the downloaded Miniforge installer into `<version>/conda`, silently and without
/// touching PATH, the registry, shortcuts or shell profiles. Leaves nothing behind when it fails.
pub async fn setup_conda(layout: &Layout, version: &str) -> Result<()> {
    let home = layout.bin("conda", version);
    let result = async {
        let installer = home.join(if cfg!(windows) { "Miniforge3.exe" } else { "Miniforge3.sh" });
        std::fs::rename(home.join("installed.txt"), &installer)?;
        let target = home.join("conda");
        if cfg!(windows) {
            // NSIS reads /D= as the rest of the command line, so it comes last and unquoted
            // (spaces are fine there), and it only takes backslashes.
            let args: Vec<String> = ["/InstallationType=JustMe", "/RegisterPython=0", "/AddToPath=0", "/NoRegistry=1", "/NoShortcuts=1", "/S"]
                .iter()
                .map(|s| s.to_string())
                .chain(std::iter::once(format!("/D={}", target.display().to_string().replace('/', "\\"))))
                .collect();
            run_quiet(&installer, &args, &home).await?;
        } else {
            let args = vec![installer.to_string_lossy().into_owned(), "-b".to_string(), "-p".to_string(), target.to_string_lossy().into_owned()];
            run_quiet(Path::new("bash"), &args, &home).await?;
        }
        let _ = std::fs::remove_file(&installer);
        if !conda_program(&home).exists() {
            return Err(Error::msg("the installer finished without installing conda"));
        }
        std::fs::write(home.join("installed.txt"), format!("Miniforge {version}\n"))?;
        Ok::<_, Error>(())
    }
    .await;
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&home);
        return Err(Error::msg(format!("Installing Miniforge failed: {e}")));
    }
    Ok(())
}

/// The executable of an installed Compass: the app bundle on macOS, otherwise the file
/// next to resources/ named after Compass.
pub fn compass_program(home: &Path) -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        return Some(home.join("MongoDB Compass.app")).filter(|p| p.exists());
    }
    std::fs::read_dir(home).ok()?.flatten().map(|e| e.path()).find(|p| {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_lowercase();
        p.is_file() && name.contains("compass") && (if cfg!(windows) { name.ends_with(".exe") && !name.contains("update") } else { !name.contains('.') })
    })
}

/// ngrok publishes a "v3 stable" download per platform, without a version number in the
/// URL or a checksum, so this download relies on HTTPS.
fn ngrok_releases() -> Vec<Release> {
    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "amd64" };
    let (os, ext, archive) = match std::env::consts::OS {
        "windows" => ("windows", "zip", Archive::Zip),
        "macos" => ("darwin", "zip", Archive::Zip),
        _ => ("linux", "tgz", Archive::TarGz),
    };
    vec![Release {
        runtime: Runtime::Ngrok,
        version: "3-stable".into(),
        url: format!("https://bin.equinox.io/c/bNyj1mQVY4c/ngrok-v3-stable-{os}-{arch}.{ext}"),
        sha256: None,
        sha512: None,
        md5: None,
        sha3_256: None,
        archive,
        label: Some("Stable channel".into()),
    }]
}

/// MinGit from Git for Windows' latest release (GitHub's SHA-256 digest checked). The tag
/// is "v2.47.1.windows.1"; the version shown is Git's own ("2.47.1", or "2.47.1.2" for a
/// Windows-only follow-up release).
async fn git_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    if !cfg!(windows) {
        return Err(Error::msg("Use your system's Git: install it with your package manager, or xcode-select --install on macOS."));
    }
    let suffix = if cfg!(target_arch = "aarch64") { "-arm64.zip" } else { "-64-bit.zip" };
    let mut releases = github_latest(client, Runtime::Git, "git-for-windows/git", Archive::Zip, |n, _| n.starts_with("MinGit-") && n.ends_with(suffix) && !n.contains("busybox")).await?;
    for r in &mut releases {
        r.version = git_version_from_tag(&r.version);
    }
    Ok(releases)
}

fn git_version_from_tag(tag: &str) -> String {
    match tag.split_once(".windows.") {
        Some((base, "1")) => base.to_string(),
        Some((base, n)) => format!("{base}.{n}"),
        None => tag.to_string(),
    }
}

async fn cloudflared_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "amd64" };
    let (name, archive) = match std::env::consts::OS {
        "windows" => (format!("cloudflared-windows-{arch}.exe"), Archive::File),
        "macos" => (format!("cloudflared-darwin-{arch}.tgz"), Archive::TarGz),
        _ => (format!("cloudflared-linux-{arch}"), Archive::File),
    };
    github_latest(client, Runtime::Cloudflared, "cloudflare/cloudflared", archive, |n, _| n == name).await
}

async fn adminer_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    // The all-drivers, all-languages single file.
    github_latest(client, Runtime::Adminer, "vrana/adminer", Archive::File, |n, v| n == format!("adminer-{v}.php"))
        .await
}

/// phpRedisAdmin publishes source releases on GitHub (no separate archives or checksums,
/// so this relies on HTTPS). Its PHP dependency, Predis, is added after install.
async fn phpredisadmin_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let latest = get_json(client, "https://api.github.com/repos/erikdubbelboer/phpRedisAdmin/releases/latest").await?;
    let tag = latest["tag_name"].as_str().ok_or_else(|| Error::msg("unexpected phpRedisAdmin release data"))?;
    Ok(vec![Release {
        runtime: Runtime::Phpredisadmin,
        version: tag.trim_start_matches('v').to_string(),
        url: format!("https://github.com/erikdubbelboer/phpRedisAdmin/archive/refs/tags/{tag}.zip"),
        sha256: None,
        sha512: None,
        md5: None,
        sha3_256: None,
        archive: Archive::Zip,
        label: None,
    }])
}

/// phpRedisAdmin talks to Redis through Predis, normally installed by Composer. LampRabit
/// adds the exact version its composer.json asks for, with a two-line autoloader, so
/// neither Composer nor PHP is needed to install it.
pub async fn add_predis(client: &reqwest::Client, layout: &Layout, version: &str) -> Result<()> {
    let home = layout.bin("phpredisadmin", version);
    let target = home.join("vendor").join("predis").join("predis");
    if target.join("autoload.php").exists() {
        return Ok(());
    }
    let composer: Value = serde_json::from_str(&std::fs::read_to_string(home.join("composer.json"))?)
        .map_err(|e| Error::msg(format!("phpRedisAdmin's composer.json: {e}")))?;
    let predis = composer["require"]["predis/predis"].as_str().unwrap_or("v2.4.1").trim_start_matches(['^', '~', '=']);
    let tag = if predis.starts_with('v') { predis.to_string() } else { format!("v{predis}") };
    let bytes = client
        .get(format!("https://github.com/predis/predis/archive/refs/tags/{tag}.zip"))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(http)?
        .bytes()
        .await
        .map_err(http)?;
    let scratch = layout.tmp().join(format!("predis-{tag}.extract"));
    let _ = std::fs::remove_dir_all(&scratch);
    zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .and_then(|mut z| z.extract(&scratch))
        .map_err(|e| Error::msg(format!("could not extract Predis: {e}")))?;
    let root = find_root(&scratch, Path::new("autoload.php"), 2).ok_or_else(|| Error::msg("the Predis archive has no autoload.php"))?;
    std::fs::create_dir_all(target.parent().unwrap())?;
    std::fs::rename(root, &target)?;
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::write(home.join("vendor").join("autoload.php"), "<?php\n// Written by LampRabit in place of Composer's autoloader.\nrequire __DIR__ . '/predis/predis/autoload.php';\n")?;
    Ok(())
}

/// getcomposer.org lists the current release and the 2.2 LTS line (the last one that
/// runs on PHP below 7.2.5), each with a published checksum.
async fn composer_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    const SITE: &str = "https://getcomposer.org";
    let index = get_json(client, &format!("{SITE}/versions")).await?;
    let mut seen = Vec::new();
    let fetches = index["stable"].as_array().into_iter().flatten().filter_map(|r| {
        let version = r["version"].as_str()?.to_string();
        let path = r["path"].as_str()?.to_string();
        if seen.contains(&version) {
            return None;
        }
        seen.push(version.clone());
        // Composer's LTS line (2.2) is the one kept for PHP 5.3–7.1, not the one to pick
        // for a new setup; the label says so, and one-click installs don't prefer it.
        let label = r["lts"].as_bool().unwrap_or(false).then(|| "LTS for PHP 5.3–7.1".to_string());
        Some(async move {
            let url = format!("{SITE}{path}");
            let sha256 = client
                .get(format!("{url}.sha256sum"))
                .send()
                .await
                .and_then(|r| r.error_for_status())
                .map_err(http)?
                .text()
                .await
                .map_err(http)?
                .split_whitespace()
                .next()
                .map(str::to_string);
            // Never install Composer without a checksum: it runs with your project's rights.
            if sha256.is_none() {
                return Err(Error::msg(format!("getcomposer.org has no checksum for Composer {version}")));
            }
            Ok(Release { runtime: Runtime::Composer, version, url, sha256, sha512: None, md5: None, sha3_256: None, archive: Archive::File, label })
        })
    });
    futures_util::future::join_all(fetches.collect::<Vec<_>>()).await.into_iter().collect()
}

/// Composer used to live directly in `bin/composer/composer.phar`; moves it into a
/// version folder like every other runtime. The version is read from the phar itself.
pub fn migrate_flat_composer(layout: &Layout) {
    let dir = layout.root().join("bin").join("composer");
    let flat = dir.join("composer.phar");
    let Ok(bytes) = std::fs::read(&flat) else { return };
    let text = String::from_utf8_lossy(&bytes);
    let version = text
        .split("const VERSION = '")
        .nth(1)
        .and_then(|s| s.split('\'').next())
        .filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-'));
    if let Some(v) = version {
        let dest = dir.join(v);
        if std::fs::create_dir_all(&dest).is_ok() && !dest.join("composer.phar").exists() {
            let _ = std::fs::rename(&flat, dest.join("composer.phar"));
        }
    }
    // Unreadable or a duplicate: a fresh copy is installed the next time it's needed.
    let _ = std::fs::remove_file(&flat);
}

/// LampRabit 0.1 ran Microsoft Garnet as Redis, with a private .NET runtime and (on
/// Windows) a separate redis-cli. Real Redis replaced it everywhere, so those folders are
/// deleted; Garnet's versions have no redis-server and already count as not installed.
/// Returns whether Garnet was found, so the user can be told to install Redis again.
pub fn remove_garnet(layout: &Layout) -> bool {
    let bin = layout.root().join("bin");
    let garnet: Vec<PathBuf> = std::fs::read_dir(bin.join("redis"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join(exe("GarnetServer")).exists() && !p.join(Runtime::Redis.marker()).exists())
        .collect();
    for dir in &garnet {
        let _ = std::fs::remove_dir_all(dir);
    }
    for old in ["dotnet", "rediscli"] {
        let _ = std::fs::remove_dir_all(bin.join(old));
    }
    !garnet.is_empty()
}

async fn phpmyadmin_releases(client: &reqwest::Client) -> Result<Vec<Release>> {
    let info = get_json(client, "https://www.phpmyadmin.net/home_page/version.json").await?;
    let version = info["version"].as_str().ok_or_else(|| Error::msg("unexpected phpMyAdmin version data"))?;
    let php = info["releases"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|r| r["version"] == version)
        .and_then(|r| r["php_versions"].as_str())
        // ">=7.2,<8.4" -> "PHP <8.4": the upper bound is what users run into.
        .map(|p| format!("PHP {}", p.rsplit(',').next().unwrap_or(p).trim()));
    let url = format!("https://files.phpmyadmin.net/phpMyAdmin/{version}/phpMyAdmin-{version}-all-languages.zip");
    let sha256 = match client.get(format!("{url}.sha256")).send().await {
        Ok(r) if r.status().is_success() => {
            r.text().await.ok().and_then(|t| t.split_whitespace().next().map(str::to_string))
        }
        _ => None,
    };
    Ok(vec![Release {
        runtime: Runtime::Phpmyadmin,
        version: version.to_string(),
        url,
        sha256,
        sha512: None,
        md5: None,
        sha3_256: None,
        archive: Archive::Zip,
        label: php,
    }])
}

pub fn installed(layout: &Layout, runtime: Runtime) -> Vec<String> {
    let dir = layout.root().join("bin").join(runtime.id());
    let mut versions: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().join(runtime.marker()).exists())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.'))
        .collect();
    versions.sort_by_key(|v| std::cmp::Reverse(version_key(v)));
    versions
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub runtime: Runtime,
    pub version: String,
    pub stage: &'static str,
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// Runtimes whose install continues after the download (npm, pip, rustup, Predis): the
/// installer reports "setup" rather than "done", and the app reports done when it ends.
pub fn has_setup(runtime: Runtime) -> bool {
    matches!(runtime, Runtime::Rust | Runtime::Phpredisadmin | Runtime::Conda) || crate::manifest::get(runtime.id()).is_some_and(|m| m.install.is_some())
}

pub async fn install(app: &AppHandle, client: &reqwest::Client, layout: &Layout, release: &Release) -> Result<()> {
    let runtime = release.runtime;
    let dest = layout.bin(runtime.id(), &release.version);
    if dest.join(runtime.marker()).exists() {
        return Ok(());
    }
    let progress = |stage, downloaded, total| {
        let _ = app.emit(
            "install-progress",
            InstallProgress { runtime, version: release.version.clone(), stage, downloaded, total },
        );
    };

    std::fs::create_dir_all(layout.tmp())?;
    let name = format!("{}-{}", runtime.id(), release.version);
    let archive_path = layout.tmp().join(format!("{name}.download"));
    let extract_dir = layout.tmp().join(format!("{name}.extract"));

    let result = async {
        let response = client.get(&release.url).send().await.and_then(|r| r.error_for_status()).map_err(http)?;
        let total = response.content_length();
        let mut file = tokio::fs::File::create(&archive_path).await?;
        let mut hasher = Sha256::new();
        let mut hasher512 = sha2::Sha512::new();
        let mut hasher_md5 = md5::Md5::new();
        let mut hasher_sha3 = sha3::Sha3_256::new();
        let mut downloaded = 0u64;
        let mut last_emit = Instant::now();
        let mut stream = response.bytes_stream();
        progress("downloading", 0, total);
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(http)?;
            hasher.update(&chunk);
            hasher512.update(&chunk);
            hasher_md5.update(&chunk);
            hasher_sha3.update(&chunk);
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            if last_emit.elapsed() > Duration::from_millis(150) {
                progress("downloading", downloaded, total);
                last_emit = Instant::now();
            }
        }
        file.flush().await?;
        drop(file);

        progress("verifying", downloaded, total);
        let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let checks = [
            (&release.sha256, hex(&hasher.finalize())),
            (&release.sha512, hex(&hasher512.finalize())),
            (&release.md5, hex(&hasher_md5.finalize())),
            (&release.sha3_256, hex(&hasher_sha3.finalize())),
        ];
        for (expected, actual) in checks {
            if let Some(expected) = expected
                && !actual.eq_ignore_ascii_case(expected) {
                    let head = std::fs::read(&archive_path).map(|b| b.into_iter().take(512).collect::<Vec<u8>>()).unwrap_or_default();
                    let page = String::from_utf8_lossy(&head).to_ascii_lowercase();
                    if page.contains("<html") || page.contains("<!doctype") {
                        return Err(Error::msg(format!("{name}: the server sent a web page instead of the file (it may block automated downloads, or be updating it). Try again in a few minutes.")));
                    }
                    return Err(Error::msg(format!("checksum mismatch for {name}: the download doesn't match its published checksum (the file may have been replaced upstream moments ago). Try again in a few minutes.")));
                }
        }

        progress("extracting", downloaded, total);
        // A single file with install steps keeps the name they expect (`source.save_as`).
        let save_as = crate::manifest::get(runtime.id()).and_then(|m| m.source.here()).and_then(|s| s.save_as.as_ref()).map(|n| PathBuf::from(n.replace("{version}", &release.version)));
        let (archive_path, extract_dir, dest, marker, kind) =
            (archive_path.clone(), extract_dir.clone(), dest.clone(), save_as.unwrap_or_else(|| runtime.marker()), release.archive);
        tokio::task::spawn_blocking(move || extract_into(&archive_path, kind, &extract_dir, &dest, &marker))
            .await
            .map_err(|e| Error::msg(e.to_string()))??;
        progress(if has_setup(runtime) { "setup" } else { "done" }, downloaded, total);
        Ok(())
    }
    .await;

    let _ = std::fs::remove_file(&archive_path);
    let _ = std::fs::remove_dir_all(&extract_dir);
    if result.is_err() {
        progress("error", 0, None);
    }
    result
}

fn extract_into(archive: &Path, kind: Archive, scratch: &Path, dest: &Path, marker: &Path) -> Result<()> {
    if scratch.exists() {
        std::fs::remove_dir_all(scratch)?;
    }
    if let Archive::File = kind {
        let target = dest.join(marker);
        std::fs::create_dir_all(target.parent().unwrap_or(dest))?;
        std::fs::copy(archive, &target)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))?;
        }
        return Ok(());
    }
    let file = std::fs::File::open(archive)?;
    match kind {
        Archive::File => unreachable!(),
        Archive::Zip => zip::ZipArchive::new(file)
            .and_then(|mut z| z.extract(scratch))
            .map_err(|e| Error::msg(format!("could not extract archive: {e}")))?,
        Archive::TarGz => tar::Archive::new(flate2::read::GzDecoder::new(file)).unpack(scratch)?,
        Archive::TarZst => {
            let decoder = ruzstd::decoding::StreamingDecoder::new(std::io::BufReader::new(file))
                .map_err(|e| Error::msg(format!("could not decompress archive: {e}")))?;
            tar::Archive::new(decoder).unpack(scratch)?
        }
        Archive::TarXz => {
            // Through a file rather than memory: unpacked, MySQL's archive is several hundred MB.
            let tar_path = scratch.with_extension("tar");
            let result = (|| {
                let mut out = std::io::BufWriter::new(std::fs::File::create(&tar_path)?);
                lzma_rs::xz_decompress(&mut std::io::BufReader::new(file), &mut out)
                    .map_err(|e| Error::msg(format!("could not decompress archive: {e}")))?;
                std::io::Write::flush(&mut out)?;
                drop(out);
                tar::Archive::new(std::fs::File::open(&tar_path)?).unpack(scratch)?;
                Ok::<_, Error>(())
            })();
            let _ = std::fs::remove_file(&tar_path);
            result?
        }
    }

    // Archives wrap their content differently (Apache24/, mariadb-x.y-winx64/, or nothing);
    // the runtime root is whichever directory contains the marker file. macOS JDKs are the
    // deepest: jdk-21.0.5+11/Contents/Home/bin/java.
    let root = find_root(scratch, marker, 3)
        .ok_or_else(|| Error::msg(format!("archive does not contain {}", marker.display())))?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if dest.exists() {
        std::fs::remove_dir_all(dest)?;
    }
    std::fs::rename(root, dest)?;
    // macOS marks what a quarantined app downloads, and Gatekeeper then blocks each binary
    // on first run ("can't be opened"). The download was checksum-checked, so the mark goes.
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("xattr").args(["-dr", "com.apple.quarantine"]).arg(dest).status();
    }
    Ok(())
}

fn find_root(dir: &Path, marker: &Path, depth: u32) -> Option<PathBuf> {
    if dir.join(marker).exists() {
        return Some(dir.to_path_buf());
    }
    if depth == 0 {
        return None;
    }
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir())
        .find_map(|e| find_root(&e.path(), marker, depth - 1))
}

pub fn remove(layout: &Layout, runtime: Runtime, version: &str) -> Result<()> {
    let dir = layout.bin(runtime.id(), version);
    if !dir.starts_with(layout.root().join("bin")) || version.contains("..") {
        return Err(Error::msg("invalid version"));
    }
    // Rename first: Windows refuses while any file inside is in use, so a version is
    // either removed completely or left untouched, never half-deleted.
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
    let trash = dir.with_file_name(format!(".removing-{version}-{stamp}"));
    std::fs::rename(&dir, &trash).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(32) {
            Error::msg(format!(
                "{} {version} is still in use by a running program. Stop the services using it (or close any terminal running it) and try again.",
                runtime.id()
            ))
        } else {
            Error::from(e)
        }
    })?;
    if std::fs::remove_dir_all(&trash).is_err() {
        // Already out of the way and invisible to LampRabit; cleaned up on a later start.
        eprintln!("could not finish deleting {}", trash.display());
    }
    Ok(())
}

/// Deletes leftovers of removals that couldn't finish (see `remove`).
pub fn clean_removed(layout: &Layout) {
    let Ok(runtimes) = std::fs::read_dir(layout.root().join("bin")) else { return };
    for runtime in runtimes.flatten() {
        let Ok(entries) = std::fs::read_dir(runtime.path()) else { continue };
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(".removing-") {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotice {
    pub runtime: Runtime,
    pub installed: String,
    pub release: Release,
}

/// Newer patch releases for installed versions: same "major.minor" line (for example
/// PHP 8.4.27 → 8.4.30), never a jump to a new minor that could change behaviour.
pub async fn updates(client: &reqwest::Client, layout: &Layout) -> Vec<UpdateNotice> {
    let checks = [Runtime::Apache, Runtime::Nginx, Runtime::Php, Runtime::Mariadb, Runtime::Mailpit, Runtime::Adminer, Runtime::Phpmyadmin, Runtime::Redis, Runtime::Composer, Runtime::Mysql, Runtime::Phpredisadmin, Runtime::Memcached, Runtime::Caddy, Runtime::Java, Runtime::Tomcat, Runtime::Frankenphp, Runtime::Postgres, Runtime::Python, Runtime::Phppgadmin, Runtime::Pgadmin, Runtime::Mongodb, Runtime::Mongosh, Runtime::Mongotools, Runtime::Compass, Runtime::Sqlite, Runtime::Clickhouse, Runtime::Neo4j, Runtime::Meilisearch, Runtime::Typesense, Runtime::Rustfs, Runtime::Ollama, Runtime::Minio, Runtime::Elasticsearch, Runtime::Opensearch, Runtime::N8n, Runtime::Rabbitmq, Runtime::Erlang, Runtime::Go, Runtime::Maven, Runtime::Gradle, Runtime::Git, Runtime::Bun, Runtime::Deno, Runtime::Uv, Runtime::Conda]
        .into_iter()
        .map(|runtime| async move {
            let installed = installed(layout, runtime);
            if installed.is_empty() {
                return Vec::new();
            }
            let Ok(catalog) = available(client, runtime, false).await else { return Vec::new() };
            let mut out = Vec::new();
            for have in &installed {
                if let Some(newer) = catalog.iter().find(|r| {
                    line(&r.version) == line(have) && version_key(&r.version) > version_key(have) && !installed.contains(&r.version)
                }) {
                    out.push(UpdateNotice { runtime, installed: have.clone(), release: newer.clone() });
                }
            }
            out
        });
    futures_util::future::join_all(checks).await.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every manifest source, against the real GitHub and npm APIs: catches asset names
    /// that don't exist. `cargo test --lib live_manifest_catalogs -- --ignored --nocapture`
    /// (set GITHUB_TOKEN: it makes ~20 API calls).
    #[tokio::test]
    #[ignore]
    async fn live_manifest_catalogs() {
        let client = client();
        let mut failed = Vec::new();
        for m in crate::manifest::built_ins().iter().filter(|m| m.source.here().is_some_and(|s| !s.lamprabit)) {
            let Some(runtime) = Runtime::from_id(&m.id) else { continue };
            match manifest_releases(&client, runtime).await {
                Ok(list) => println!("{:<14} {}", m.id, list.iter().map(|r| format!("{} [{}{}]", r.version, if r.sha256.is_some() { "sha256" } else if r.sha512.is_some() { "sha512" } else if r.md5.is_some() { "md5" } else { "unchecked" }, r.label.as_ref().map(|l| format!(", {l}")).unwrap_or_default())).collect::<Vec<_>>().join("  ")),
                Err(e) if m.unavailable_here().is_some() => println!("{:<14} unavailable here: {e}", m.id),
                Err(e) => failed.push(format!("{}: {e}", m.id)),
            }
        }
        assert!(failed.is_empty(), "{failed:#?}");
    }

    #[test]
    fn reads_git_for_windows_tags() {
        assert_eq!(git_version_from_tag("2.47.1.windows.1"), "2.47.1");
        assert_eq!(git_version_from_tag("2.47.1.windows.2"), "2.47.1.2");
        assert_eq!(git_version_from_tag("2.48.0-rc1.windows.1"), "2.48.0-rc1");
    }

}
