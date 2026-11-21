# Service manifests

Every service LampRabit runs (web servers, databases, caches, search engines, queues, mail catching, AI) is described by a YAML file, its *manifest*: where the downloads come from, which ports it uses, which files it needs before it starts, how it is started, stopped and checked, and how apps connect to it. LampRabit reads the manifest and does the rest. The built-in manifests are in [`src-tauri/services/`](../src-tauri/services/), one per service.

Community services will use the same format. They'll come from the **LampRabit/Services** repository: you open a pull request with a `service.yaml`, it's reviewed and tested on Windows, Linux and macOS, and once it's merged the service shows up in the app's marketplace and on the website. Until that lands, this page is the reference for the built-in manifests and for anyone preparing one.

## A complete example

```yaml
schema: 1
id: meilisearch
name: Meilisearch
category: search
blurb: Search engine
origin: Official Meilisearch Community builds from GitHub (checksum checked)
homepage: https://www.meilisearch.com
license: MIT

source:
  github: meilisearch/meilisearch
  versions: newest
  assets:
    windows-x64: { file: meilisearch-windows-amd64.exe, archive: file }
    linux-x64: { file: meilisearch-linux-amd64, archive: file }
    linux-arm64: { file: meilisearch-linux-aarch64, archive: file }
    macos-x64: { file: meilisearch-macos-amd64, archive: file }
    macos-arm64: { file: meilisearch-macos-apple-silicon, archive: file }
binary: "meilisearch{{exe}}"

ports:
  http: { default: 7700 }

dirs: ["{{data}}"]

run:
  program: "{{home}}/meilisearch{{exe}}"
  args: [--db-path, "{{data}}/data.ms", --http-addr, "127.0.0.1:{{port.http}}", --env, development, --no-analytics]
  cwd: "{{data}}"

health: { http: { path: /health } }

connection:
  url: "http://127.0.0.1:{{port.http}}"
  env:
    MEILISEARCH_HOST: "http://127.0.0.1:{{port.http}}"
    MEILISEARCH_KEY: ""
```

## Fields

### About the service

| Field | Required | What it is |
|---|---|---|
| `schema` | yes | The format's version: `1`. |
| `id` | yes | Lowercase letters, digits and dashes (at most 40). It names the service's folders: `bin/<id>/<version>/`, `etc/<id>/`, `data/<id>/`, `logs/<id>/`, `tmp/<id>/`. |
| `name` | yes | Shown on its card. |
| `category` | yes | The Services page group: `web`, `databases`, `search`, `storage`, `queues`, `ai`, `cache`, `tools`, `sharing` (and `languages`, `packages`). |
| `blurb` | yes | A few words under the name ("Search engine"). |
| `origin` | | Where the downloads come from and anything worth knowing, shown on the Services page. |
| `description`, `homepage`, `license`, `maintainers`, `icon` | | For the marketplace. `icon` is a file next to the manifest. |
| `revision` | | The manifest's own version (default 1). Raise it with every change. It has nothing to do with the software's version. |
| `requires.lamprabit` | | The oldest LampRabit that understands the manifest, as `"0.8.0"`. Older apps refuse it and say why. |
| `unavailable` | | Why it can't run on some platforms: `{ value: "Typesense has no Windows build. Use Meilisearch.", on: [windows] }`. |
| `suggests` | | Ids of services that go well with it (a web interface for its data). |

### Where the downloads come from: `source`

`origin` is only the sentence shown on the Services page. **`source` is what LampRabit downloads from**, and manifests find new versions on their own: when the software publishes a release, LampRabit offers it with no change to the manifest. A new manifest revision is only needed when the way the service is installed or run changes.

A source is either GitHub or npm:

| Field | What it is |
|---|---|
| `github` | `owner/repo`. Versions come from its releases (or tags), and files from its releases unless `url` says otherwise. |
| `npm` | A package on npm's registry: its latest version's tarball, checked against npm's SHA-512 (`checksum: npm`). |
| `versions` | Which versions are offered. `latest` (the default) is the release GitHub marks as latest. `newest` is the newest recent release that has this platform's file, for projects that sometimes ship a release without one platform's build. `per-major` is the newest release of each major version from `min` on (PostgreSQL 14–18). `per-line` is the newest patch of each `major.minor` line, the `keep` newest lines (4 by default). `newest-tag` is the newest `x.y.z` tag, for projects that tag on GitHub but publish elsewhere. Drafts and pre-releases are skipped. |
| `min`, `keep` | For `per-major` and `per-line`. |
| `tags` | Only tags ending with one of these suffixes, which the version drops; the value is the label shown next to it: `{ -lts: LTS, -stable: Stable }`. |
| `assets` | The file for each platform. The key is `<os>-<arch>` (`windows-x64`, `linux-arm64`, `macos-arm64`…), an OS (`macos`), or `unix` (Linux and macOS); the most exact key wins. `file` is the file's name, where `{version}` stands for the release's version. `archive` is `zip`, `targz`, `tarxz`, `tarzst` or `file` (a single program). **Quote names that contain `{version}`**, since YAML reads `{` as the start of a map. |
| `url` | Download from the vendor's own site instead of the GitHub release: `"https://artifacts.elastic.co/downloads/elasticsearch/{file}"` (`{version}`, `{file}`). |
| `checksum` | `github-digest` (the default): the SHA-256 GitHub reports for each release file. A download from `url` needs the vendor's checksum file instead: `{ sha256: "{url}.sha256" }` or `{ sha512: "{url}.sha512" }`. |
| `label` | A note shown next to the versions: `"Large download"`, `"PostgreSQL {major}"`, `"Node {node}"` (npm: the Node.js the package asks for). |

**Nothing is installed unchecked.** A release without a checksum is left out of the list, and when none is left the Services page says why.

A service can use different sources per platform: give a list, and the first whose `on` matches wins.

```yaml
source:
  - on: [windows]
    github: redis-windows/redis-windows
    versions: per-line
    assets:
      windows-x64: { file: "Redis-{version}-Windows-x64-msys2.zip", archive: zip }
  - { lamprabit: true }
```

A platform with no file in `assets` can't install the service; give it an `unavailable` message that says what to use instead.

`binary` (at the top level, not in `source`) is the file inside the install whose presence means the install is complete. A service whose binary is missing doesn't start.

### Ports

```yaml
ports:
  amqp: { default: 5672, label: AMQP }
  management: { default: 15672, label: Management console }
```

Each port has a name (lowercase letters and underscores) and a default. The **first** port is the main one: it's shown on the service's card and is what `health` checks unless told otherwise. Every port is checked before the service starts, and a port that's taken stops the start with a message naming who holds it. Community services must use ports above 1023, since lower ones need administrator rights on Linux and macOS. Avoid 9000–9099, which PHP's FastCGI workers use.

When a community service is installed, LampRabit gives each of its ports the default, or the next port that no other service is set to use and nothing is listening on, so two services never share one. The user can change them on the service's Settings card; they are saved in `settings.toml` under `[ports.extra]` as `"<id>.<port name>"`.

### Before it starts: `data`, `dirs`, `files`

- `data`: where `{{data}}` points. `shared` (the default) is `data/<id>`. `per-major` is `data/<id>-<major>`, for software whose files only work with the major version that wrote them. `storage` is the storage folder from Settings, which can be anywhere, for object storage.
- `dirs`: folders created before every start. Each must start with a folder placeholder.
- `files`: written before every start, in order, so the service always runs with LampRabit's settings. Each entry does one thing:

```yaml
files:
  # Write a file (relative paths are inside etc/<id>).
  - path: rabbitmq.conf
    content: |
      listeners.tcp.default = 127.0.0.1:{{port.amqp}}
  # Copy a file or folder from the install (into etc/<id> unless `to` says otherwise).
  - copy: "{{home}}/config"
  - { copy: "{{home}}/conf/server-logs.xml", optional: true }
  # Take the install's key=value file and replace some keys.
  - path: neo4j.conf
    merge:
      from: "{{home}}/conf/neo4j.conf"
      set:
        server.http.listen_address: ":{{port.http}}"
```

Files can only be written inside the service's own folders. A path with `..`, or an absolute path that doesn't start with a folder placeholder, is refused.

### Options the user can change: `settings`

```yaml
settings:
  memory:
    label: Memory limit
    description: How much memory Memcached may use; the oldest items go first when it's full.
    type: number
    default: 128
    min: 16
    max: 65536
    unit: MB
  appendonly:
    label: Write every change to disk
    type: toggle
    default: false
run:
  args: [-m, "{{setting.memory}}", { values: [--appendonly, "yes"], if: setting.appendonly }]
```

Each setting shows in the service's **Settings** card on the Services page, with an input for its `type`:

| `type` | Input | Extra fields |
|---|---|---|
| `text` | One line of text (no line breaks, at most 200 characters) | |
| `number` | A whole number | `min`, `max`, `unit` |
| `toggle` | A switch: `true` or `false` | |
| `choice` | One of `options` | `options` |

`{{setting.<name>}}` is the value, anywhere placeholders work (arguments, environment, file contents). `if: setting.<name>` keeps an argument, a value or a file only while the setting is on (a toggle that's on, or any other setting that isn't empty or `0`); `if: "!setting.<name>"` is the opposite. LampRabit checks every value against its type before saving it, keeps only values that differ from the default (so a new default reaches users), and restarts the service when it's running.

### Starting and stopping: `run`, `stop`

```yaml
run:
  program: "{{home}}/bin/neo4j{{script}}"
  args: [console]
  cwd: "{{data}}"            # the install folder when not given
  path: ["{{needs.node.bin}}"] # folders put in front of PATH
  env:
    NEO4J_CONF: "{{etc}}"
    JAVA_HOME: "{{needs.java.home}}"
stop:
  command: ["{{home}}/sbin/rabbitmqctl{{script}}", stop, -n, lamprabit@localhost]
```

- `program` must be inside the install (`{{home}}/…`) or a runtime the service `needs` (`{{needs.java.bin}}/…`). There is no shell: `args` are passed to the program as they are, so `&&`, pipes and `$VAR` mean nothing.
- The service must run **in the foreground**, since LampRabit follows the process it started. Turn off daemon modes (`--daemonize no`, `-D FOREGROUND`).
- It must listen on **127.0.0.1** only: LampRabit is for local development, and a service open to the network is a hole in the user's machine.
- Its output goes to `logs/<id>-output.log`. Set `log` to the name of the service's own log in `logs/` when it keeps one.
- `stop.command` asks the service to shut down cleanly. Without it, LampRabit ends the process tree after a grace period. `if_installed: mongosh` runs the command only when that other runtime is installed, and `{{runtime.mongosh.home}}` points at it.

### Is it up: `health`

Exactly one of:

```yaml
health: { http: { path: /health } }               # any answer but a server error
health: { http: { port: ui, path: /livez } }      # on another of its ports
health: { tcp: {} }                               # the port accepts connections
health: { line: { send: "PING\r\n", expect: "+PONG" } }
```

Services that take a while to start (a JVM, Python, Node) should set `behaviour: { slow: true }`. They stay *Starting* until the check passes, so a bare TCP check, which answers before the service is ready, doesn't fit them. `behaviour: { heavy: true }` leaves a service out of Start all, the tray and start-on-launch, for services that need a lot of memory.

### What it runs on: `needs`, `system_needs`

```yaml
needs:
  - { runtime: java, min: "21", why: Neo4j runs on the active Java }
  - { runtime: erlang, on: [windows], why: RabbitMQ runs on Erlang }
system_needs:
  - { command: erl, on: [unix], hint: "Install Erlang with your system's packages (sudo apt install erlang-nox), then start RabbitMQ again." }
```

`needs` names a runtime LampRabit installs: `java`, `erlang`, `python`, or `node` (NVM's). The service won't start until it's installed in a version at least `min`, and the error says what to install. `{{needs.<id>.home}}` and `{{needs.<id>.bin}}` point at it. `system_needs` looks for a command on PATH that LampRabit can't provide and shows `hint` when it's missing.

### How apps reach it: `connection`, `ui`

```yaml
connection:
  url: "redis://127.0.0.1:{{port.redis}}"
  env:
    REDIS_HOST: 127.0.0.1
    REDIS_PORT: "{{port.redis}}"
ui: { domain: n8n, port: http }
```

`connection` is what Home's **Connect your app** card and the service's card copy: a URL and `.env` lines, in the order written. Use `127.0.0.1` rather than `localhost`, because MySQL clients read `localhost` as "use the Unix socket". `ui` serves a web interface at `https://<domain>.test` through the web server, and `{{ui.url}}` is that address (for software that needs to know its public URL).

## Placeholders

| Placeholder | Value |
|---|---|
| `{{home}}` | The install: `bin/<id>/<version>` |
| `{{data}}` | Its data folder (see `data`) |
| `{{etc}}`, `{{logs}}`, `{{tmp}}` | `etc/<id>`, `logs/<id>`, `tmp/<id>` |
| `{{version}}`, `{{major}}` | The active version, and its first number |
| `{{port.<name>}}` | One of its ports, as set in Settings |
| `{{exe}}`, `{{script}}` | `.exe` and `.bat` on Windows, nothing elsewhere |
| `{{needs.<id>.home}}`, `{{needs.<id>.bin}}` | A runtime it `needs` |
| `{{runtime.<id>.home}}` | The runtime named by `stop.if_installed` |
| `{{ui.url}}` | Its `https://<domain>.test` address |
| `{{setting.<name>}}` | One of its `settings` |

Folder paths: whatever follows a folder placeholder up to the next space is a path, so `{{etc}}/my.ini` and `--dir={{data}}/x` get this OS's separator (`\` on Windows). Add `|slash` to keep forward slashes (`{{data|slash}}`), which config files and programs built with MSYS2 want. An unknown placeholder is an error when the manifest is read, not when the service starts.

## Per-platform values

Any text can be a list of `{ value, on }` where the first that matches wins, and any argument or environment variable can apply to some platforms only:

```yaml
program:
  - { value: "{{home}}/bin/ollama", on: [linux] }
  - { value: "{{home}}/ollama{{exe}}" }          # no `on`: everywhere else
args: ["--defaults-file={{etc}}/my.ini", { value: --console, on: [windows] }]
env:
  LD_LIBRARY_PATH: { value: "{{home}}/lib/compat", on: [linux] }
```

`on` takes `windows`, `linux`, `macos`, `unix` (Linux and macOS), `x64`, `arm64`, or `<os>-<arch>`.

## Adding and testing a service

To use a service that LampRabit doesn't ship, choose **Add a service** on the Services page and pick its `service.yaml`. LampRabit shows what the file downloads, how the download is checked, what it runs and which ports and files it uses; nothing is added until you approve, because nobody has reviewed a local file. The file is copied to `<LampRabit folder>/services/<id>/service.yaml` (with its icon, when `icon` names a file next to it) and loaded on every start. That folder is kept on uninstall and moves with the LampRabit folder. Adding a file with the same `id` again updates the service (stop it first). **Remove service** on its card deletes the file once its installed versions are removed; its data folder stays.

While writing a manifest:

```sh
lamprabit service check service.yaml   # is it valid, and what does it do
lamprabit service try service.yaml     # install, start, health check, stop, in a throwaway folder
```

`try` runs the same checks as LampRabit's own CI smoke test, so a manifest that passes it on Windows, Linux and macOS is ready to share.

## Built-in services

LampRabit's own manifests may use `builtin:` for the parts its code still handles, because they can't be described as files and commands: `ports` (the web servers' ports come from Settings → Web server), `prepare` (vhosts, PHP pools, database data folders), `launch` (Tomcat's classpath, PostgreSQL started through `pg_ctl` when LampRabit has administrator rights) and `external` (an Ollama installed outside LampRabit). They may also use `source: { builtin: true }` for catalogs that need code (MariaDB's REST API, MySQL's, MongoDB's, Tomcat's mirror listing, pgAdmin's PyPI wheels, and the Windows builds of Apache and Nginx, which are scraped from web pages), `source: { lamprabit: true }` for LampRabit's own CI builds, `checksum: none` (Typesense publishes no checksums), `setting:` on a port to name its field in Settings → Ports, and the `mysql` and `postgres` health checks. Community manifests can't use any of these.

## Checks

Every manifest is checked when it's read, and a broken one is refused with the reason: unknown fields (usually a typo), unknown placeholders or platforms, a port listed twice, a health check that names a missing port, paths that leave the service's folders, programs outside the install. The built-in manifests are checked by `cargo test` (`manifest::tests`), so a broken one never ships.
