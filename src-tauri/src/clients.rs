//! Database clients (`connects` in a manifest, KarotX for one): after such a service starts,
//! LampRabit adds a connection to it for every local database that's installed, through the
//! client's own HTTP API, and updates the ones it added before. It finds its own by name, so
//! connections the user made are never touched. Format reference: docs/services.md.

use serde::Deserialize;
use serde_json::Value;

use crate::{
    config::Settings,
    error::{Error, Result},
    layout::Layout,
    manifest::{self, Manifest, Ordered},
    services::ServiceKind,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connects {
    /// The URL schemes LampRabit's databases publish (`connection.url`) and the client's
    /// name for each one's driver: `{ mariadb: mysql, postgresql: postgres }`.
    pub drivers: Ordered<String>,
    /// Lists the client's connections (GET, a JSON list).
    pub list: String,
    /// Adds one (POST).
    pub create: String,
    /// Changes one (PUT); `{{id}}` is its id.
    pub update: String,
    /// Where a listed connection keeps its id and its name.
    #[serde(default = "id_field")]
    pub id_field: String,
    #[serde(default = "name_field")]
    pub name_field: String,
    /// The name LampRabit gives each connection, which is how it finds its own again.
    pub name: String,
    /// What's sent for each database. `{{db.name}}`, `{{db.title}}` (the service's name),
    /// `{{db.service}}`, `{{db.driver}}`, `{{db.host}}`, `{{db.port}}`, `{{db.user}}` and
    /// `{{db.database}}` fill it in; a value that is exactly `{{db.port}}` is sent as a number.
    pub body: Value,
}

fn id_field() -> String {
    "id".into()
}

fn name_field() -> String {
    "name".into()
}

const DB: &[&str] = &["name", "title", "service", "driver", "host", "port", "user", "database"];

/// The client's API is on this machine, and its texts name only what `sync` fills in.
pub fn check(c: &Connects, ports: &[String]) -> std::result::Result<(), String> {
    for (what, url) in [("list", &c.list), ("create", &c.create), ("update", &c.update)] {
        if !url.starts_with("http://127.0.0.1:") && !url.starts_with("http://localhost:") {
            return Err(format!("connects.{what} must be an address on this computer (http://127.0.0.1:…)"));
        }
    }
    if c.drivers.0.is_empty() {
        return Err("connects.drivers lists no databases".into());
    }
    let mut texts: Vec<&str> = vec![&c.list, &c.create, &c.update, &c.name];
    fn strings<'a>(v: &'a Value, out: &mut Vec<&'a str>) {
        match v {
            Value::String(s) => out.push(s),
            Value::Array(a) => a.iter().for_each(|x| strings(x, out)),
            Value::Object(o) => o.values().for_each(|x| strings(x, out)),
            _ => {}
        }
    }
    strings(&c.body, &mut texts);
    for text in texts {
        let mut rest = text;
        while let Some(open) = rest.find("{{") {
            let close = rest[open..].find("}}").map(|i| open + i).ok_or_else(|| format!("connects: unclosed {{{{ in \"{text}\""))?;
            let name = rest[open + 2..close].trim();
            let ok = name == "id" || name.strip_prefix("db.").is_some_and(|f| DB.contains(&f)) || name.strip_prefix("port.").is_some_and(|p| ports.iter().any(|q| q == p));
            if !ok {
                return Err(format!("connects: unknown placeholder {{{{{name}}}}}"));
            }
            rest = &rest[close + 2..];
        }
    }
    Ok(())
}

/// One installed local database, as the client will see it.
#[derive(Debug, Clone, PartialEq)]
pub struct Database {
    pub service: String,
    pub title: String,
    pub driver: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub database: String,
}

/// The installed services whose connection address uses a scheme the client knows.
pub fn databases(m: &Manifest, c: &Connects, settings: &Settings, layout: &Layout) -> Vec<Database> {
    let mut out = Vec::new();
    for kind in ServiceKind::all() {
        let other = kind.manifest();
        if other.id == m.id || settings.versions.get(&other.id).is_none() {
            continue;
        }
        let Some(info) = kind.connection(settings, layout) else { continue };
        let Ok(url) = reqwest::Url::parse(&info.url) else { continue };
        let Some(driver) = c.drivers.get(url.scheme()) else { continue };
        out.push(Database {
            service: other.id.clone(),
            title: other.name.clone(),
            driver: driver.clone(),
            host: url.host_str().unwrap_or("127.0.0.1").to_string(),
            port: url.port().unwrap_or_default(),
            user: url.username().to_string(),
            database: url.path().trim_start_matches('/').to_string(),
        });
    }
    out
}

/// The client's name for a database's connection, and what to send for it.
pub fn request(m: &Manifest, c: &Connects, settings: &Settings, layout: &Layout, db: &Database) -> Result<(String, Value)> {
    let mut ctx = manifest::Ctx::any(m, settings, layout);
    for (k, v) in [("title", &db.title), ("service", &db.service), ("driver", &db.driver), ("host", &db.host), ("user", &db.user), ("database", &db.database)] {
        ctx.set(format!("db.{k}"), v.clone());
    }
    ctx.set("db.port", db.port.to_string());
    let name = ctx.render(&c.name)?;
    ctx.set("db.name", name.clone());
    fn fill(v: &Value, ctx: &manifest::Ctx, port: u16) -> Result<Value> {
        Ok(match v {
            Value::String(s) if s.trim() == "{{db.port}}" => Value::from(port),
            Value::String(s) => Value::String(ctx.render(s)?),
            Value::Array(a) => Value::Array(a.iter().map(|x| fill(x, ctx, port)).collect::<Result<_>>()?),
            Value::Object(o) => Value::Object(o.iter().map(|(k, x)| Ok((k.clone(), fill(x, ctx, port)?))).collect::<Result<_>>()?),
            other => other.clone(),
        })
    }
    Ok((name, fill(&c.body, &ctx, db.port)?))
}

/// Adds or updates the client's connection for every installed local database; returns
/// how many it sent.
pub async fn sync(client: &reqwest::Client, m: &Manifest, settings: &Settings, layout: &Layout) -> Result<usize> {
    let Some(c) = &m.connects else { return Ok(0) };
    let ctx = manifest::Ctx::any(m, settings, layout);
    let list_url = ctx.render(&c.list)?;
    let existing: Value = client.get(&list_url).send().await.and_then(|r| r.error_for_status()).map_err(|e| Error::msg(format!("{} didn't list its connections: {e}", m.name)))?.json().await.map_err(|e| Error::msg(e.to_string()))?;
    let existing = existing.as_array().cloned().unwrap_or_default();
    let mut sent = 0;
    for db in databases(m, c, settings, layout) {
        let (name, body) = request(m, c, settings, layout, &db)?;
        let id = existing.iter().find(|e| e[c.name_field.as_str()] == name.as_str()).and_then(|e| e[c.id_field.as_str()].as_str().map(str::to_string));
        let req = match id {
            Some(id) => {
                let mut ctx = manifest::Ctx::any(m, settings, layout);
                ctx.set("id", id);
                client.put(ctx.render(&c.update)?)
            }
            None => client.post(ctx.render(&c.create)?),
        };
        req.json(&body).send().await.and_then(|r| r.error_for_status()).map_err(|e| Error::msg(format!("{} didn't take the {} connection: {e}", m.name, db.title)))?;
        sent += 1;
    }
    Ok(sent)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// KarotX ships as a bundled manifest (community rules, no code) and gets a connection
    /// for each installed database, from the addresses their manifests publish.
    #[test]
    fn karotx_lists_every_installed_database() {
        let kind = ServiceKind::from_id("karotx").expect("KarotX is bundled");
        let m = kind.manifest();
        assert!(manifest::is_bundled("karotx") && m.builtin.is_empty());
        let c = m.connects.as_ref().unwrap();
        let mut settings = Settings::default();
        for (id, v) in [("mariadb", "11.8.5"), ("postgres", "18.1.0"), ("redis", "8.4.0"), ("mailpit", "1.34.0"), ("karotx", "0.1.0")] {
            settings.versions.set(id, Some(v.into()));
        }
        let layout = Layout::new(std::env::temp_dir().join("lr-clients-test"));
        let dbs = databases(m, c, &settings, &layout);
        let mut ids: Vec<&str> = dbs.iter().map(|d| d.service.as_str()).collect();
        ids.sort();
        assert_eq!(ids, ["mariadb", "postgres", "redis"], "databases only, installed only, not itself");

        let maria = dbs.iter().find(|d| d.service == "mariadb").unwrap();
        let (name, body) = request(m, c, &settings, &layout, maria).unwrap();
        assert_eq!(name, "MariaDB (LampRabit)");
        let conn = &body["connection"];
        assert_eq!((conn["driver"].as_str(), conn["preset"].as_str(), conn["user"].as_str()), (Some("mysql"), Some("mariadb"), Some("root")));
        assert_eq!(conn["port"], serde_json::json!(3306), "a number, not text");
        assert_eq!(conn["name"].as_str(), Some("MariaDB (LampRabit)"));

        let pg = dbs.iter().find(|d| d.service == "postgres").unwrap();
        let (_, body) = request(m, c, &settings, &layout, pg).unwrap();
        assert_eq!((body["connection"]["driver"].as_str(), body["connection"]["database"].as_str()), (Some("postgres"), Some("postgres")));
    }

    #[test]
    fn a_client_api_must_be_on_this_computer() {
        let c: Connects = serde_yaml_ng::from_str("drivers: { mysql: mysql }\nlist: https://example.com/c\ncreate: http://127.0.0.1:1/c\nupdate: http://127.0.0.1:1/c/{{id}}\nname: x\nbody: {}\n").unwrap();
        assert!(check(&c, &[]).unwrap_err().contains("this computer"));
        let c: Connects = serde_yaml_ng::from_str("drivers: { mysql: mysql }\nlist: http://127.0.0.1:1/c\ncreate: http://127.0.0.1:1/c\nupdate: http://127.0.0.1:1/c/{{id}}\nname: \"{{db.secret}}\"\nbody: {}\n").unwrap();
        assert!(check(&c, &[]).unwrap_err().contains("unknown placeholder"));
    }
}
