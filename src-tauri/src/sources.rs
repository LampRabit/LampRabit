//! Download lists from a vendor's JSON API (`source.json`) or a web page (`source.page`),
//! for services whose releases aren't GitHub releases or npm packages: MariaDB's and
//! MongoDB's APIs, PyPI, docker-library's MySQL list, Apache Lounge, nginx.org and Apache's
//! mirror. The fetching is in `releases`; everything else works on the fetched text, so
//! tests feed it saved answers. Format reference: docs/services.md.

use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

use crate::{
    error::{Error, Result},
    manifest::{self, Checksum, Found, Hash, Manifest, Ordered, Source, Versions},
    runtimes::Archive,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonSource {
    pub url: String,
    /// Path to the releases in the answer (`versions`; `*` for an object's values). Empty:
    /// the answer is one release.
    #[serde(default)]
    pub items: String,
    /// Only items whose fields have these values (`*` in a value matches anything).
    #[serde(default, rename = "where")]
    pub filter: Ordered<String>,
    /// A further request per item (`{item.<path>}` in its address), whose `items` are the
    /// releases; the first request's item is then `parent`.
    #[serde(default)]
    pub then: Option<Follow>,
    /// Path to a release's version.
    pub version: String,
    /// Path to a release's files, and the values the right one has.
    #[serde(default)]
    pub files: Option<String>,
    #[serde(default)]
    pub file: Ordered<String>,
    /// The download's address, a template (`{file.archive.url}`).
    pub download: String,
    /// Hashes the answer gives (templates); otherwise `checksum` names a file.
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub sha512: Option<String>,
    #[serde(default)]
    pub label: Option<Label>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Follow {
    pub url: String,
    #[serde(default)]
    pub items: String,
}

/// A release's label: a template, or a field's value looked up in `map` (`else` for any
/// other value; without it, the value itself).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Label {
    Text(String),
    Map {
        from: String,
        #[serde(default)]
        map: Ordered<String>,
        #[serde(default, rename = "else")]
        otherwise: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageSource {
    /// The page; `{variant}` takes each of `variants` in turn.
    pub url: String,
    /// A regular expression for the links (`href` values) that are downloads, with a
    /// `version` group and, optionally, a `label` group.
    pub link: String,
    /// One lookup per value (Tomcat's release lines), keeping the newest version of each.
    #[serde(default)]
    pub variants: Vec<Variant>,
    /// The download's address when it isn't the link (`{version}`, `{variant}`, `{asset}`).
    #[serde(default)]
    pub download: Option<String>,
    /// Labels for the versions found, newest first (nginx.org's Mainline, Stable).
    #[serde(default)]
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub value: String,
    #[serde(default)]
    pub label: Option<String>,
}

/// One case of a manifest's `vars`: its value where every condition holds.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VarCase {
    #[serde(default)]
    pub value: Option<String>,
    /// Shown instead when this case matches: a system the service has no build for
    /// (`{distro}` is the distribution's name).
    #[serde(default)]
    pub unavailable: Option<String>,
    #[serde(default)]
    pub on: Vec<String>,
    /// Linux distributions by their /etc/os-release ID…
    #[serde(default)]
    pub distro: Vec<String>,
    /// …or one they're built on (ID_LIKE),
    #[serde(default)]
    pub like: Option<String>,
    /// its codename (UBUNTU_CODENAME, VERSION_CODENAME),
    #[serde(default)]
    pub codename: Option<String>,
    /// and its version from this one on (VERSION_ID).
    #[serde(default)]
    pub from: Option<String>,
}

/// Checks the parts that can be checked without fetching anything.
pub fn check(src: &Source) -> std::result::Result<(), String> {
    let https = |what: &str, t: &str| if t.starts_with("https://") { Ok(()) } else { Err(format!("source.{what} \"{t}\" must be an https:// address")) };
    if let Some(j) = &src.json {
        https("json.url", &j.url)?;
        if let Some(f) = &j.then {
            https("json.then.url", &f.url)?;
        }
        if j.version.is_empty() || j.download.is_empty() {
            return Err("source.json needs version and download".into());
        }
    }
    if let Some(p) = &src.page {
        https("page.url", &p.url)?;
        let re = Regex::new(&p.link).map_err(|e| format!("source.page.link isn't a valid regular expression: {e}"))?;
        if !re.capture_names().flatten().any(|n| n == "version") {
            return Err("source.page.link needs a (?P<version>…) group".into());
        }
        if let Some(d) = &p.download {
            https("page.download", d)?;
        }
    }
    Ok(())
}

/// The releases a `json` or `page` source offers on this machine.
pub async fn releases(client: &reqwest::Client, m: &Manifest, src: &Source) -> Result<Vec<Found>> {
    let vars = Vars::new(m)?;
    if let Some(j) = &src.json {
        let answer = get_json(client, &vars.fill(&j.url, &Ctx::default())?).await?;
        let mut entries = Vec::new();
        for (key, item) in items(&answer, &j.items) {
            if !matches_all(&j.filter, &item, &vars, &Ctx::default())? {
                continue;
            }
            match &j.then {
                None => entries.push(Entry { key, parent: Value::Null, item: item.clone() }),
                Some(follow) => {
                    let ctx = Ctx { item: Some(item), ..Ctx::default() };
                    // A line that doesn't answer is left out, like a release without this platform's file.
                    let Ok(next) = get_json(client, &vars.fill(&follow.url, &ctx)?).await else { continue };
                    for (k, sub) in items(&next, &follow.items) {
                        entries.push(Entry { key: k.or(key.clone()), parent: item.clone(), item: sub.clone() });
                    }
                }
            }
        }
        return from_json(m, src, j, &vars, entries);
    }
    let p = src.page.as_ref().ok_or_else(|| manifest::no_download(m))?;
    let variants: Vec<Option<&Variant>> = if p.variants.is_empty() { vec![None] } else { p.variants.iter().map(Some).collect() };
    let mut pages = Vec::new();
    for variant in variants {
        let ctx = Ctx { variant: variant.map(|v| v.value.as_str()), ..Ctx::default() };
        let url = vars.fill(&p.url, &ctx)?;
        let text = client.get(&url).send().await.and_then(|r| r.error_for_status()).map_err(|e| Error::msg(format!("Couldn't read {url}: {e}")))?.text().await.map_err(|e| Error::msg(e.to_string()))?;
        pages.push((variant, url, text));
    }
    from_pages(m, src, p, &vars, pages)
}

async fn get_json(client: &reqwest::Client, url: &str) -> Result<Value> {
    client.get(url).send().await.and_then(|r| r.error_for_status()).map_err(|e| Error::msg(format!("Couldn't read {url}: {e}")))?.json().await.map_err(|e| Error::msg(format!("{url} didn't answer with JSON: {e}")))
}

/// One release candidate from a JSON source: its item, the item of the first request when
/// there was a second, and its key when the list was an object.
pub struct Entry {
    pub key: Option<String>,
    pub parent: Value,
    pub item: Value,
}

pub fn from_json(m: &Manifest, src: &Source, j: &JsonSource, vars: &Vars, entries: Vec<Entry>) -> Result<Vec<Found>> {
    let asset = src.asset();
    let mut found = Vec::new();
    for e in &entries {
        let Some(version) = text(lookup(&e.item, &j.version, e.key.as_deref())).filter(|v| !v.is_empty()) else { continue };
        let version = version.trim_start_matches('v').to_string();
        let base = Ctx { item: Some(&e.item), parent: Some(&e.parent), key: e.key.as_deref(), version: Some(&version), ..Ctx::default() };
        let file: Option<Value> = match &j.files {
            None => Some(e.item.clone()),
            Some(path) => {
                let list = lookup(&e.item, path, None);
                let mut picked = None;
                for f in list.as_ref().and_then(Value::as_array).into_iter().flatten() {
                    if matches_all(&j.file, f, vars, &base)? {
                        picked = Some(f.clone());
                        break;
                    }
                }
                picked
            }
        };
        let Some(file) = file else { continue };
        for name in candidates(asset, &version) {
            let ctx = Ctx { file: Some(&file), asset: Some(&name), ..base };
            let url = https(&vars.fill(&j.download, &ctx)?);
            if url.is_empty() {
                continue;
            }
            let hash = |t: &Option<String>| -> Result<Option<String>> { Ok(t.as_ref().map(|t| vars.fill(t, &ctx)).transpose()?.filter(|h| !h.is_empty())) };
            let label = match &j.label {
                None => None,
                Some(Label::Text(t)) => Some(vars.fill(t, &ctx)?),
                Some(Label::Map { from, map, otherwise }) => {
                    let value = text(lookup_ctx(&ctx, from)).unwrap_or_default();
                    match map.get(&value) {
                        Some(l) => Some(vars.fill(l, &ctx)?),
                        None => match otherwise {
                            Some(o) => Some(vars.fill(o, &ctx)?),
                            None => Some(value).filter(|v| !v.is_empty()),
                        },
                    }
                }
            };
            found.push(finish(m, src, asset, vars, &ctx, url, hash(&j.sha256)?, hash(&j.sha512)?, label)?);
        }
    }
    Ok(choose(src, found))
}

pub fn from_pages(m: &Manifest, src: &Source, p: &PageSource, vars: &Vars, pages: Vec<(Option<&Variant>, String, String)>) -> Result<Vec<Found>> {
    let re = Regex::new(&p.link).map_err(|e| Error::msg(format!("{}: source.page.link: {e}", m.id)))?;
    let asset = src.asset();
    let mut found = Vec::new();
    for (variant, page_url, page) in pages {
        let mut here = Vec::new();
        for link in hrefs(&page) {
            let Some(c) = re.captures(link) else { continue };
            let Some(version) = c.name("version").map(|v| v.as_str().to_string()) else { continue };
            let captured = c.name("label").map(|l| l.as_str().to_string());
            let link = absolute(&page_url, link);
            for name in candidates(asset, &version) {
                let ctx = Ctx { version: Some(&version), variant: variant.map(|v| v.value.as_str()), asset: Some(&name), link: Some(&link), ..Ctx::default() };
                let url = match &p.download {
                    Some(d) => https(&vars.fill(d, &ctx)?),
                    None => https(&link),
                };
                let label = variant.and_then(|v| v.label.clone()).or(captured.clone());
                here.push(finish(m, src, asset, vars, &ctx, url, None, None, label)?);
            }
        }
        // With variants, each keeps its newest version; without, the source's strategy.
        found.extend(if variant.is_some() { newest_only(here) } else { choose(src, here) });
    }
    found.sort_by_key(|f| std::cmp::Reverse(crate::version::numbers(&f.version)));
    let mut seen: Vec<String> = Vec::new();
    for f in found.iter_mut() {
        if !seen.contains(&f.version) {
            seen.push(f.version.clone());
        }
        if f.label.is_none() {
            f.label = p.labels.get(seen.len() - 1).cloned();
        }
    }
    Ok(found)
}

/// A release from its address, hashes and label, with its checksum file and archive type.
#[allow(clippy::too_many_arguments)]
fn finish(m: &Manifest, src: &Source, asset: Option<&manifest::Asset>, vars: &Vars, ctx: &Ctx, url: String, sha256: Option<String>, sha512: Option<String>, label: Option<String>) -> Result<Found> {
    let ctx = Ctx { url: Some(&url), ..*ctx };
    let checksum_file = match &src.checksum {
        Checksum::Sha256(t) => Some((Hash::Sha256, vars.fill(t, &ctx)?)),
        Checksum::Sha512(t) => Some((Hash::Sha512, vars.fill(t, &ctx)?)),
        Checksum::Md5(t) => Some((Hash::Md5, vars.fill(t, &ctx)?)),
        _ => None,
    };
    let archive = asset
        .map(|a| a.archive)
        .or(src.archive)
        .or_else(|| archive_of(&url))
        .ok_or_else(|| Error::msg(format!("{}: can't tell what kind of archive {url} is; give source.archive", m.id)))?;
    Ok(Found { version: ctx.version.unwrap_or_default().to_string(), url, archive, sha256, sha512, md5: None, checksum_file, label })
}

/// The asset file names to try, in order (none: one candidate, `{asset}` empty).
fn candidates(asset: Option<&manifest::Asset>, version: &str) -> Vec<String> {
    match asset {
        None => vec![String::new()],
        Some(a) => std::iter::once(&a.file).chain(&a.fallbacks).map(|f| f.replace("{version}", version)).collect(),
    }
}

/// Keeps the versions the source's strategy asks for, newest first. Several entries for
/// one version (its fallback files) all stay, in order.
pub fn choose(src: &Source, found: Vec<Found>) -> Vec<Found> {
    let key = |v: &str| crate::version::numbers(v);
    let mut versions: Vec<String> = Vec::new();
    for f in &found {
        if !versions.contains(&f.version) {
            versions.push(f.version.clone());
        }
    }
    versions.sort_by_key(|v| std::cmp::Reverse(key(v)));
    let keep: Vec<String> = match src.versions {
        Versions::All => versions,
        Versions::Latest | Versions::Newest | Versions::NewestTag => versions.into_iter().take(1).collect(),
        Versions::PerMajor | Versions::PerLine => {
            let parts = if src.versions == Versions::PerMajor { 1 } else { 2 };
            let mut lines: Vec<Vec<u64>> = Vec::new();
            let mut out = Vec::new();
            for v in versions {
                let k = key(&v);
                if src.versions == Versions::PerMajor && src.min.is_some_and(|min| k.first().copied().unwrap_or(0) < min) {
                    continue;
                }
                let line: Vec<u64> = k.into_iter().take(parts).collect();
                if !lines.contains(&line) {
                    lines.push(line);
                    out.push(v);
                }
            }
            if src.versions == Versions::PerLine {
                out.truncate(src.keep.unwrap_or(4));
            }
            out
        }
    };
    let mut out: Vec<Found> = found.into_iter().filter(|f| keep.contains(&f.version)).collect();
    out.sort_by_key(|f| std::cmp::Reverse(key(&f.version)));
    out
}

fn newest_only(found: Vec<Found>) -> Vec<Found> {
    let Some(top) = found.iter().map(|f| crate::version::numbers(&f.version)).max() else { return found };
    found.into_iter().filter(|f| crate::version::numbers(&f.version) == top).collect()
}

/// The hash in a checksum file of any layout: the first word of the right length that is
/// all hex digits ("<hash>  <file>", a bare hash, or a listing with labels).
pub fn find_hash(text: &str, kind: Hash) -> Option<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .find(|w| w.len() == kind.hex_len() && w.chars().all(|c| c.is_ascii_hexdigit()))
        .map(str::to_ascii_lowercase)
}

fn archive_of(url: &str) -> Option<Archive> {
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    if path.ends_with(".zip") {
        Some(Archive::Zip)
    } else if path.ends_with(".tar.gz") || path.ends_with(".tgz") {
        Some(Archive::TarGz)
    } else if path.ends_with(".tar.xz") {
        Some(Archive::TarXz)
    } else if path.ends_with(".tar.zst") {
        Some(Archive::TarZst)
    } else {
        None
    }
}

/// Vendors' lists sometimes give http:// addresses for files they also serve over HTTPS.
fn https(url: &str) -> String {
    match url.strip_prefix("http://") {
        Some(rest) => format!("https://{rest}"),
        None => url.to_string(),
    }
}

fn hrefs(page: &str) -> impl Iterator<Item = &str> {
    page.split("href=\"").skip(1).filter_map(|s| s.split('"').next())
}

fn absolute(page: &str, link: &str) -> String {
    if link.starts_with("http://") || link.starts_with("https://") {
        return link.to_string();
    }
    let origin_end = page.find("://").map(|i| i + 3).and_then(|i| page[i..].find('/').map(|j| i + j)).unwrap_or(page.len());
    if link.starts_with('/') {
        format!("{}{link}", &page[..origin_end])
    } else {
        let dir = &page[..page.rfind('/').filter(|&i| i >= origin_end).map_or(page.len(), |i| i + 1)];
        let sep = if dir.ends_with('/') { "" } else { "/" };
        format!("{dir}{sep}{link}")
    }
}

/// The items at `path`: the values of an array or object (`*` steps into each), with their
/// keys when they come from an object.
pub fn items<'a>(answer: &'a Value, path: &str) -> Vec<(Option<String>, &'a Value)> {
    let mut current: Vec<(Option<String>, &Value)> = vec![(None, answer)];
    let spread = |list: Vec<(Option<String>, &'a Value)>| -> Vec<(Option<String>, &'a Value)> {
        list.into_iter()
            .flat_map(|(_, v)| -> Vec<(Option<String>, &'a Value)> {
                match v {
                    Value::Array(a) => a.iter().map(|x| (None, x)).collect(),
                    Value::Object(o) => o.iter().map(|(k, x)| (Some(k.clone()), x)).collect(),
                    _ => vec![],
                }
            })
            .collect()
    };
    let steps: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
    for step in &steps {
        current = if *step == "*" { spread(current) } else { current.into_iter().filter_map(|(k, v)| v.get(*step).map(|x| (k, x))).collect() };
    }
    // A path that ends on a list means its entries.
    if steps.last().is_some_and(|s| *s != "*") && current.len() == 1 && current[0].1.is_array() {
        current = spread(current);
    }
    current
}

/// A dotted path inside a value (`archive.url`, `files.0.name`); `@key` is the item's key.
fn lookup<'a>(v: &'a Value, path: &str, key: Option<&str>) -> Option<Value> {
    if path == "@key" {
        return key.map(|k| Value::String(k.to_string()));
    }
    let mut cur: &'a Value = v;
    for step in path.split('.').filter(|s| !s.is_empty()) {
        cur = match cur {
            Value::Array(a) => a.get(step.parse::<usize>().ok()?)?,
            other => other.get(step)?,
        };
    }
    Some(cur.clone())
}

fn text(v: Option<Value>) -> Option<String> {
    match v? {
        Value::String(s) => Some(s),
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

/// What templates can name while one release is built.
#[derive(Clone, Copy, Default)]
pub struct Ctx<'a> {
    item: Option<&'a Value>,
    parent: Option<&'a Value>,
    file: Option<&'a Value>,
    key: Option<&'a str>,
    version: Option<&'a str>,
    variant: Option<&'a str>,
    asset: Option<&'a str>,
    link: Option<&'a str>,
    url: Option<&'a str>,
}

fn lookup_ctx(ctx: &Ctx, name: &str) -> Option<Value> {
    let (root, rest) = name.split_once('.').unwrap_or((name, ""));
    match root {
        "item" => lookup(ctx.item?, rest, ctx.key),
        "parent" => lookup(ctx.parent?, rest, None),
        "file" => lookup(ctx.file?, rest, None),
        "@key" => ctx.key.map(|k| Value::String(k.into())),
        _ => lookup(ctx.item?, name, ctx.key),
    }
}

/// The manifest's `vars`, resolved for this machine once per catalog.
pub struct Vars {
    values: Vec<(String, std::result::Result<String, String>)>,
}

impl Vars {
    pub fn new(m: &Manifest) -> Result<Self> {
        let release = if cfg!(target_os = "linux") { std::fs::read_to_string("/etc/os-release").unwrap_or_default() } else { String::new() };
        Ok(Self { values: m.vars.iter().map(|(name, cases)| (name.clone(), resolve(m, name, cases, &release))).collect() })
    }

    /// Fills `{…}` in a template: `{version}`, `{major}`, `{line}`, `{key}`, `{asset}`,
    /// `{variant}`, `{url}`, `{link}`, `{var.<name>}`, `{item.<path>}`, `{parent.<path>}`,
    /// `{file.<path>}`. A missing field is empty; an unknown name is an error.
    pub fn fill(&self, template: &str, ctx: &Ctx) -> Result<String> {
        let mut out = String::new();
        let mut rest = template;
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let close = rest[open..].find('}').map(|c| open + c).ok_or_else(|| Error::msg(format!("unclosed {{ in \"{template}\"")))?;
            let name = &rest[open + 1..close];
            let numbers = || ctx.version.map(crate::version::numbers).unwrap_or_default();
            let value = match name {
                "version" => ctx.version.unwrap_or_default().to_string(),
                "major" => numbers().first().map(u64::to_string).unwrap_or_default(),
                "line" => numbers().iter().take(2).map(u64::to_string).collect::<Vec<_>>().join("."),
                "key" => ctx.key.unwrap_or_default().to_string(),
                "asset" | "file" => ctx.asset.unwrap_or_default().to_string(),
                "variant" => ctx.variant.unwrap_or_default().to_string(),
                "url" => ctx.url.unwrap_or_default().to_string(),
                "link" => ctx.link.unwrap_or_default().to_string(),
                _ if name.starts_with("var.") => {
                    let var = &name[4..];
                    match self.values.iter().find(|(n, _)| n == var) {
                        Some((_, Ok(v))) => v.clone(),
                        Some((_, Err(why))) => return Err(Error::msg(why.clone())),
                        None => return Err(Error::msg(format!("{{{name}}}: no var named {var}"))),
                    }
                }
                _ if ["item.", "parent.", "file."].iter().any(|p| name.starts_with(p)) => text(lookup_ctx(ctx, name)).unwrap_or_default(),
                _ => return Err(Error::msg(format!("{{{name}}} isn't something a source can use"))),
            };
            out.push_str(&value);
            rest = &rest[close + 1..];
        }
        out.push_str(rest);
        Ok(out)
    }
}

fn resolve(m: &Manifest, name: &str, cases: &[VarCase], release: &str) -> std::result::Result<String, String> {
    let field = |key: &str| release.lines().find_map(|l| l.strip_prefix(&format!("{key}="))).map(|v| v.trim_matches('"').to_string()).unwrap_or_default();
    let (id, like, version) = (field("ID"), field("ID_LIKE"), field("VERSION_ID"));
    let codename = Some(field("UBUNTU_CODENAME")).filter(|c| !c.is_empty()).unwrap_or_else(|| field("VERSION_CODENAME"));
    for case in cases {
        let holds = manifest::matches(&case.on)
            && (case.distro.is_empty() || case.distro.contains(&id))
            && case.like.as_ref().is_none_or(|l| like.split_whitespace().any(|w| w == l))
            && case.codename.as_ref().is_none_or(|c| *c == codename)
            && case.from.as_ref().is_none_or(|f| crate::version::numbers(&version) >= crate::version::numbers(f));
        if !holds {
            continue;
        }
        if let Some(why) = &case.unavailable {
            let distro = Some(field("PRETTY_NAME")).filter(|n| !n.is_empty()).unwrap_or_else(|| "this system".into());
            return Err(why.replace("{distro}", &distro));
        }
        return case.value.clone().ok_or_else(|| format!("{}: var {name}: a case needs value or unavailable", m.id));
    }
    Err(format!("{} has no download for {} ({name}).", m.name, manifest::platform()))
}

fn matches_all(filter: &Ordered<String>, v: &Value, vars: &Vars, ctx: &Ctx) -> Result<bool> {
    for (path, want) in filter.iter() {
        let have = text(lookup(v, path, ctx.key)).unwrap_or_default();
        if !glob(&vars.fill(want, ctx)?, &have) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// `*` matches any run of characters; everything else must match exactly.
fn glob(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !text.starts_with(first) || !text.ends_with(last) || text.len() < first.len() + last.len() {
        return false;
    }
    let mut at = first.len();
    let end = text.len() - last.len();
    for middle in &parts[1..parts.len() - 1] {
        match text[at..end].find(middle) {
            Some(i) => at += i + middle.len(),
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs_and_hashes() {
        assert!(glob("*winx64.zip", "mariadb-11.4.2-winx64.zip"));
        assert!(!glob("*winx64.zip", "mariadb-11.4.2-winx64-debugsymbols.zip"));
        assert!(glob("Stable", "Stable") && !glob("Stable", "Alpha"));
        assert!(glob("a*b*c", "aXXbYYc") && !glob("a*b*c", "acb"));
        let apache = "httpd-2.4.65-250724-Win64-VS17.zip\r\nSHA256 Checksum:\r\n\r\nABCDEF0123456789abcdef0123456789ABCDEF0123456789abcdef0123456789\r\n";
        assert_eq!(find_hash(apache, Hash::Sha256).as_deref(), Some("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"));
        assert_eq!(find_hash("d41d8cd98f00b204e9800998ecf8427e  mysql.zip", Hash::Md5).as_deref(), Some("d41d8cd98f00b204e9800998ecf8427e"));
    }

    #[test]
    fn links_become_absolute() {
        assert_eq!(absolute("https://nginx.org/en/download.html", "/download/nginx-1.29.3.zip"), "https://nginx.org/download/nginx-1.29.3.zip");
        assert_eq!(absolute("https://dlcdn.apache.org/tomcat/tomcat-11/", "v11.0.14/"), "https://dlcdn.apache.org/tomcat/tomcat-11/v11.0.14/");
        assert_eq!(absolute("https://a.org/x/", "https://b.org/y.zip"), "https://b.org/y.zip");
    }

    /// The source of a built-in manifest that has this kind, whatever this platform is.
    fn source_of(id: &str, page: bool) -> (&'static Manifest, &'static Source) {
        let m = manifest::get(id).unwrap();
        let src = m.source.all().into_iter().find(|s| if page { s.page.is_some() } else { s.json.is_some() }).unwrap();
        (m, src)
    }

    fn entries(answer: &Value, path: &str) -> Vec<Entry> {
        items(answer, path).into_iter().map(|(key, v)| Entry { key, parent: Value::Null, item: v.clone() }).collect()
    }

    #[test]
    fn mysql_lines_from_docker_library() {
        let (m, src) = source_of("mysql", false);
        let j = src.json.as_ref().unwrap();
        let answer = serde_json::json!({"8.0": {"version": "8.0.44"}, "8.4": {"version": "8.4.7"}, "innovation": {"version": "9.5.0"}});
        let found = from_json(m, src, j, &Vars::new(m).unwrap(), entries(&answer, &j.items)).unwrap();
        let first = found.iter().find(|f| f.version == "9.5.0").unwrap();
        assert!(first.url.starts_with("https://cdn.mysql.com/Downloads/MySQL-9.5/mysql-9.5.0-"), "{}", first.url);
        assert_eq!(first.label.as_deref(), Some("Innovation"));
        assert_eq!(found.iter().find(|f| f.version == "8.4.7").unwrap().label.as_deref(), Some("LTS"));
        assert_eq!(first.checksum_file, Some((Hash::Md5, format!("{}.md5", first.url))));
        assert_eq!(found[0].version, "9.5.0", "newest first");
    }

    // MariaDB's source only covers Windows and x64 Linux (macOS uses LampRabit's builds).
    #[cfg(any(windows, all(target_os = "linux", target_arch = "x86_64")))]
    #[test]
    fn mariadb_series_from_its_rest_api() {
        let (m, src) = source_of("mariadb", false);
        let j = src.json.as_ref().unwrap();
        let index = serde_json::json!({"major_releases": [
            {"release_id": "11.8", "release_status": "Stable", "release_support_type": "Long Term Support"},
            {"release_id": "12.1", "release_status": "Stable", "release_support_type": "Short Term Support"},
            {"release_id": "12.2", "release_status": "Alpha", "release_support_type": "Short Term Support"}]});
        let file = |n: String| serde_json::json!({"file_name": n, "file_download_url": format!("http://downloads.mariadb.org/f/{n}"), "checksum": {"sha256sum": "ab12"}});
        let latest = |v: &str| {
            let files = vec![file(format!("mariadb-{v}-winx64-debugsymbols.zip")), file(format!("mariadb-{v}-winx64.zip")), file(format!("mariadb-{v}-linux-systemd-x86_64.tar.gz"))];
            let mut releases = serde_json::Map::new();
            releases.insert(v.to_string(), serde_json::json!({"release_id": v, "files": files}));
            serde_json::json!({ "releases": releases })
        };
        let vars = Vars::new(m).unwrap();
        let mut list = Vec::new();
        for (key, item) in items(&index, &j.items) {
            if !matches_all(&j.filter, item, &vars, &Ctx::default()).unwrap() {
                continue;
            }
            let next = latest(if item["release_id"] == "11.8" { "11.8.5" } else { "12.1.2" });
            for (k, sub) in items(&next, &j.then.as_ref().unwrap().items) {
                list.push(Entry { key: k.or(key.clone()), parent: item.clone(), item: sub.clone() });
            }
        }
        let found = from_json(m, src, j, &vars, list).unwrap();
        assert_eq!(found.iter().map(|f| f.version.as_str()).collect::<Vec<_>>(), ["12.1.2", "11.8.5"]);
        assert_eq!(found[1].label.as_deref(), Some("LTS"));
        assert_eq!(found[0].label.as_deref(), Some("Short Term Support"));
        assert!(found[1].url.starts_with("https://") && !found[1].url.contains("debug"), "{}", found[1].url);
        assert_eq!(found[1].sha256.as_deref(), Some("ab12"));
    }

    #[test]
    fn nginx_versions_from_its_download_page() {
        let (m, src) = source_of("nginx", true);
        let page = ["1.29.3", "1.28.0", "1.26.3", "1.24.0", "1.22.1"].iter().map(|v| format!("<a href=\"/download/nginx-{v}.zip\">x</a> <a href=\"/download/nginx-{v}.tar.gz\">x</a>")).collect::<String>();
        let found = from_pages(m, src, src.page.as_ref().unwrap(), &Vars::new(m).unwrap(), vec![(None, "https://nginx.org/en/download.html".into(), page)]).unwrap();
        assert_eq!(found.iter().map(|f| f.version.as_str()).collect::<Vec<_>>(), ["1.29.3", "1.28.0", "1.26.3", "1.24.0"]);
        assert_eq!(found[0].url, "https://nginx.org/download/nginx-1.29.3.zip");
        assert_eq!((found[0].label.as_deref(), found[1].label.as_deref(), found[2].label.as_deref()), (Some("Mainline"), Some("Stable"), None));
        assert!(found[0].checksum_file.is_none());
    }

    #[test]
    fn tomcat_keeps_the_newest_of_each_line_from_the_mirror() {
        let (m, src) = source_of("tomcat", true);
        let p = src.page.as_ref().unwrap();
        let listing = |vs: &[&str]| vs.iter().map(|v| format!("<a href=\"v{v}/\">v{v}/</a>")).collect::<String>() + "<a href=\"../\">Parent</a>";
        let pages = vec![
            (Some(&p.variants[0]), "https://dlcdn.apache.org/tomcat/tomcat-11/".to_string(), listing(&["11.0.13", "11.0.14"])),
            (Some(&p.variants[1]), "https://dlcdn.apache.org/tomcat/tomcat-10/".to_string(), listing(&["10.1.48"])),
        ];
        let found = from_pages(m, src, p, &Vars::new(m).unwrap(), pages).unwrap();
        assert_eq!(found.iter().map(|f| f.version.as_str()).collect::<Vec<_>>(), ["11.0.14", "10.1.48"]);
        let ext = if cfg!(windows) { "zip" } else { "tar.gz" };
        assert_eq!(found[0].url, format!("https://dlcdn.apache.org/tomcat/tomcat-11/v11.0.14/bin/apache-tomcat-11.0.14.{ext}"));
        assert_eq!(found[0].label.as_deref(), Some("Jakarta EE 11 · Java 17+"));
        assert_eq!(found[0].checksum_file, Some((Hash::Sha512, format!("{}.sha512", found[0].url))));
    }

    #[test]
    fn apache_lounge_build_and_its_checksum_file() {
        let (m, src) = source_of("apache", true);
        let page = "<a href=\"/download/VS18/binaries/httpd-2.4.65-250724-Win64-VS18.zip\">httpd</a> <a href=\"/download/VS18/binaries/httpd-2.4.65-250724-Win64-VS18.zip.txt\">txt</a>";
        let found = from_pages(m, src, src.page.as_ref().unwrap(), &Vars::new(m).unwrap(), vec![(None, "https://www.apachelounge.com/download/".into(), page.into())]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].version, "2.4.65");
        assert_eq!(found[0].url, "https://www.apachelounge.com/download/VS18/binaries/httpd-2.4.65-250724-Win64-VS18.zip");
        assert_eq!(found[0].label.as_deref(), Some("VS18"));
        assert_eq!(found[0].checksum_file, Some((Hash::Sha256, format!("{}.txt", found[0].url))));
    }

    #[test]
    fn pgadmin_wheel_from_pypi() {
        let (m, src) = source_of("pgadmin", false);
        let j = src.json.as_ref().unwrap();
        let answer = serde_json::json!({"info": {"version": "9.10"}, "urls": [
            {"filename": "pgadmin4-9.10.tar.gz", "url": "https://files.pythonhosted.org/a.tar.gz", "digests": {"sha256": "aa"}},
            {"filename": "pgadmin4-9.10-py3-none-any.whl", "url": "https://files.pythonhosted.org/b.whl", "digests": {"sha256": "bb"}}]});
        let found = from_json(m, src, j, &Vars::new(m).unwrap(), entries(&answer, &j.items)).unwrap();
        assert_eq!((found[0].version.as_str(), found[0].url.as_str(), found[0].sha256.as_deref()), ("9.10", "https://files.pythonhosted.org/b.whl", Some("bb")));
        assert!(matches!(found[0].archive, Archive::File));
    }

    #[test]
    fn mongodb_build_follows_the_linux_distribution() {
        // The manifest's Linux cases, checked against os-release files on any platform.
        let m = manifest::get("mongodb").unwrap();
        let cases: Vec<VarCase> = m.vars.get("target").unwrap().iter().filter(|c| c.on == ["linux"]).map(|c| VarCase { on: vec![], ..c.clone() }).collect();
        let os = |id: &str, version: &str, extra: &str| format!("ID={id}\nVERSION_ID=\"{version}\"\n{extra}");
        let target = |text: String| resolve(m, "target", &cases, &text);
        assert_eq!(target(os("ubuntu", "24.04", "")).unwrap(), "ubuntu2404");
        assert_eq!(target(os("ubuntu", "25.10", "")).unwrap(), "ubuntu2404", "between releases: the older build");
        assert_eq!(target(os("debian", "13", "")).unwrap(), "debian13");
        assert_eq!(target(os("rocky", "9.4", "")).unwrap(), "rhel93");
        assert_eq!(target(os("linuxmint", "22", "ID_LIKE=\"ubuntu debian\"\nUBUNTU_CODENAME=jammy")).unwrap(), "ubuntu2204");
        assert!(target(os("arch", "", "PRETTY_NAME=\"Arch Linux\"")).unwrap_err().contains("Arch Linux"));
    }

    #[test]
    fn items_walk_arrays_objects_and_keys() {
        let v: Value = serde_json::json!({"versions": [{"v": 1}, {"v": 2}], "lines": {"8.4": {"version": "8.4.7"}, "innovation": {"version": "9.5.0"}}});
        assert_eq!(items(&v, "versions").len(), 2);
        let lines = items(&v, "lines.*");
        assert_eq!(lines[1].0.as_deref(), Some("innovation"));
        assert_eq!(items(&v, "").len(), 1);
        assert_eq!(lookup(&v, "versions.1.v", None), Some(serde_json::json!(2)));
    }
}
