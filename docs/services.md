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

A source is GitHub, npm, a vendor's JSON API (`json`) or a web page (`page`):

| Field | What it is |
|---|---|
| `github` | `owner/repo`. Versions come from its releases (or tags), and files from its releases unless `url` says otherwise. |
| `npm` | A package on npm's registry: its latest version's tarball, checked against npm's SHA-512 (`checksum: npm`). |
| `versions` | Which versions are offered. `latest` (the default) is the release GitHub marks as latest. `newest` is the newest recent release that has this platform's file, for projects that sometimes ship a release without one platform's build. `per-major` is the newest release of each major version from `min` on (PostgreSQL 14–18). `per-line` is the newest patch of each `major.minor` line, the `keep` newest lines (4 by default). `newest-tag` is the newest `x.y.z` tag, for projects that tag on GitHub but publish elsewhere. Drafts and pre-releases are skipped. |
| `min`, `keep` | For `per-major` and `per-line`. |
| `skip` | Versions never offered (`json` and `page` sources), for a release published broken upstream; the next one down is offered instead. Give the source every release (`items: releases.*` for PyPI) so there is one to fall back to. |
| `prefix` | Text before the version in every tag, dropped; tags without it are skipped. Bun tags `bun-v1.2.3`, so `prefix: bun-`. |
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

#### A vendor's API: `json`

For software whose releases are listed by a JSON API rather than GitHub or npm (MariaDB, MongoDB, PyPI, docker-library's MySQL list):

```yaml
source:
  json:
    url: https://downloads.mongodb.org/full.json
    items: versions                         # path to the releases; `*` steps into an object's values
    where: { production_release: "true" }   # keep items whose fields match (`*` is a wildcard)
    version: version                        # path to a release's version
    files: downloads                        # path to its files…
    file: { target: "{var.target}", arch: "{var.arch}" }   # …and the values the right one has
    download: "{file.archive.url}"
    sha256: "{file.archive.sha256}"         # a hash the answer gives (or sha512)
    label: { from: lts_release, map: { "true": LTS }, else: "MongoDB {major}" }
  versions: per-major
  min: 7
```

- `then: { url: ".../{item.release_id}/latest/", items: "releases.*" }` makes one more request per item (MariaDB lists its series first, then each one's latest release); the first request's item is then `parent` (`parent.release_support_type`).
- Without `items`, the answer is one release (PyPI's `info.version`).
- Templates (`download`, hashes, labels, filter values) can use `{version}`, `{major}`, `{line}` (`8.4` of 8.4.7), `{key}` (an item's key when the list was an object; `@key` in a path), `{asset}` (the platform's file from `assets`), `{item.<path>}`, `{parent.<path>}`, `{file.<path>}` and `{var.<name>}`. Paths are dotted (`archive.url`, `urls.0.digests.sha256`).
- `label` is a template, or `{ from: <path>, map: { value: label }, else: <label> }`; without `else` the value itself is shown.

#### A web page: `page`

For a download page or a mirror's folder listing (Apache Lounge, nginx.org, Apache's mirror):

```yaml
source:
  page:
    url: "https://dlcdn.apache.org/tomcat/tomcat-{variant}/"
    link: '^v(?P<version>\d+\.\d+\.\d+)/$'      # a regular expression for the links that are releases
    variants:                                    # one lookup per value, keeping the newest of each
      - { value: "11", label: "Jakarta EE 11 · Java 17+" }
      - { value: "10", label: "Jakarta EE 10 · Java 11+" }
    download: "https://dlcdn.apache.org/tomcat/tomcat-{variant}/v{version}/bin/{asset}"
  assets:
    windows: { file: "apache-tomcat-{version}.zip", archive: zip }
    unix: { file: "apache-tomcat-{version}.tar.gz", archive: targz }
  checksum: { sha512: "{url}.sha512" }
```

`link` needs a `version` group and may have a `label` group. Without `download`, the link itself is the download. `labels: [Mainline, Stable]` names the versions found, newest first. Without `variants`, `versions` picks among the versions found (`latest`, `per-line`…).

#### Shared by `json` and `page`

- `versions: all` keeps every release the list gives.
- **Checksums:** a hash from the answer, or a checksum file: `{ sha256 | sha512 | md5: "{url}.sha256" }`. LampRabit finds the hash in the file whatever its layout. MD5 is accepted only because some vendors (MySQL) publish nothing stronger.
- `archive` gives the archive type when no `assets` entry does; otherwise it comes from the address (`.zip`, `.tar.gz`, `.tgz`, `.tar.xz`, `.tar.zst`).
- `fallbacks` on an asset lists more file names to try, in order, when the first has no published checksum (MySQL's macOS archives are named after the macOS they're built for).
- Addresses from an answer that start with `http://` are fetched over HTTPS.

#### Values per platform or distribution: `vars`

At the top level, `vars` gives values that differ by platform or Linux distribution, used as `{var.<name>}`. The first case that matches wins:

```yaml
vars:
  target:
    - { value: windows, on: [windows-x64] }
    - { value: ubuntu2404, on: [linux], distro: [ubuntu], from: "24.04" }
    - { value: ubuntu2404, on: [linux], like: ubuntu }                    # Mint, Pop!_OS…
    - { on: [linux], unavailable: "There's no build for {distro}." }
```

`distro` is the `ID` from /etc/os-release, `like` one of its `ID_LIKE` words, `codename` its `UBUNTU_CODENAME` (or `VERSION_CODENAME`), `from` the oldest `VERSION_ID`. A case with `unavailable` stops the download list with that message (`{distro}` is the distribution's name).

A platform with no file in `assets` can't install the service; give it an `unavailable` message that says what to use instead.

`binary` (at the top level, not in `source`) is the file inside the install whose presence means the install is complete. A service whose binary is missing doesn't start.

### Ports

```yaml
ports:
  amqp: { default: 5672, label: AMQP }
  management: { default: 15672, label: Management console }
```

Each port has a name (lowercase letters and underscores) and a default. The **first** port is the main one: it's shown on the service's card and is what `health` checks unless told otherwise. Every port is checked before the service starts, and a port that's taken stops the start with a message naming who holds it. Community services must use ports above 1023, since lower ones need administrator rights on Linux and macOS. Avoid 9000–9099, which PHP's FastCGI workers use.

A port can have a `label` ("HTTP", "Console"; leave it out for a service's only port) and a `hint`, a few words on what uses it, shown under its field. When a service is installed, LampRabit gives each of its ports the default, or the next port that no other service is set to use and nothing is listening on, so two services never share one. The user can change them on the service's Settings card and in Settings → Ports; they are saved in `settings.toml` under `[ports.services]` as `"<id>.<port name>"`.

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

### Around the start: `init`, `guard`, `libraries`, `after_start`

For services with a data folder that needs setting up once (a database's system tables), data that only works with some versions, or a step once the service answers:

```yaml
# Commands that set up a new data folder. `done` lists paths whose presence means it's set up;
# a folder that isn't empty but isn't set up is left alone (it may be someone's data), and a
# failed set-up removes the folder so the next start tries again.
init:
  done: ["{{data}}/mysql"]
  creates_folder: false          # true: the program creates the folder itself (mysqld refuses one that exists)
  run:
    - { program: "{{home}}/bin/mariadb-install-db{{exe}}", args: ["--datadir={{data}}"], on: [windows] }
    - { program: "{{home}}/scripts/mariadb-install-db", args: [--no-defaults, "--datadir={{data}}"], on: [unix] }

# The release line that last used the data, kept in `marker` once `when` exists. Starting an
# older line on newer data is refused (it can damage it); after a move to a newer line,
# `upgrade` runs once the service answers, and the marker moves on when it succeeds.
guard:
  marker: "{{data}}/lamprabit-version"
  when: "{{data}}/mysql"
  line: 2                        # parts of the version that make a line: 10.11
  upgrade: ["{{home}}/bin/mariadb-upgrade{{exe}}", "--defaults-file={{etc}}/my.ini", --user=root]

# Linux: system libraries the program needs that some distributions renamed. A link under
# the old name goes in `dir` (put it on LD_LIBRARY_PATH in run.env); `hint` says what to
# install when libraries are still missing ({missing} lists them).
libraries:
  check: "{{home}}/bin/mysqld"
  dir: "{{home}}/lib/compat"
  renamed: { libaio.so.1: libaio.so.1t64 }
  hint: "MySQL needs system libraries that aren't installed ({missing}). …"

# Commands once the service answers its health check.
after_start:
  - { program: "{{home}}/bin/psql{{exe}}", args: [-h, 127.0.0.1, -p, "{{port.postgres}}", -U, postgres, -c, "…"] }
```

These commands run programs from the install (or a needed runtime), with the service's `run.env`. `data: per-line` keeps a data folder per `major.minor` (`data/mysql-8.4`), for software that can't move its files between release lines.

### Installing, one-time steps and outside copies: `install`, `once`, `external`

```yaml
# After the download is unpacked: steps that finish the install (a Python app's virtual
# environment, npm's packages). `clean` is removed first; the install counts (its `binary`
# marker is written) once every step worked, and a failure removes it.
install:
  clean: ["{{home}}/venv"]
  run:
    - { program: "{{needs.python.bin}}/python3", args: [-m, venv, "{{home}}/venv"], on: [unix] }
    - { program: "{{needs.node.bin}}/npm", args: [install, --prefix, "{{home}}", "n8n@{{version}}"], path: ["{{needs.node.bin}}"], on: [unix] }

# Commands run before a start while their marker is missing and `if` holds; a failure is a
# warning, not a failed start.
once:
  - marker: "{{data}}/lamprabit-servers.json"
    if: installed.postgres
    run:
      - { program: "{{home}}/venv/bin/python3", args: ["{{var.package}}/setup.py", load-servers, "{{tmp}}/servers.json"], cwd: "{{var.package}}" }
```

A command may have `path` (folders put in front of PATH) and `cwd`. A single-file download with install steps is saved as `source.save_as` (`pgadmin4-{version}-py3-none-any.whl`). `{{ports.<id>.<port>}}` is another service's port, and `if: installed.<id>` holds when that service is installed. A var case can be a `glob` instead of a `value`: the first existing path that matches (`{{home}}/venv/lib/python*/site-packages/pgadmin4`).

LampRabit's own manifests may also use `external` (a copy installed by the software's own installer, used while LampRabit has none; `paths` may read `{{env.<VAR>}}`) and `run.elevated` (Windows with administrator rights: LampRabit starts the service with these arguments and stands in for it, as PostgreSQL needs).

### Backups: `backup`

A database lists, backs up and restores its databases for the Backups panel with its own tools:

```yaml
backup:
  extension: dump                      # backups/<id>/<database>-<date>.dump
  list:                                # one database per line of output
    program: "{{home}}/bin/psql{{exe}}"
    args: [-h, 127.0.0.1, -p, "{{port.postgres}}", -U, postgres, -tAc, "SELECT datname FROM pg_database"]
  skip: [template0, template1]         # never offered
  dump:
    - { program: "{{home}}/bin/pg_dump{{exe}}", args: [-F, c, -f, "{{file}}", -d, "{{database}}"] }
  restore:
    - { program: "{{home}}/bin/createdb{{exe}}", args: ["{{database}}"], if_missing: true }
    - { program: "{{home}}/bin/pg_restore{{exe}}", args: [--clean, -d, "{{database}}", "{{file}}"] }
```

`{{database}}` is the database (letters, digits, `-` and `_` only), `{{file}}` the backup file, `{{original}}` the database a backup was made from (restoring under another name), and `{{runtime.<id>.home}}` another installed runtime's folder (MongoDB's tools; the panel says to install it when it's missing). A step can feed a file to the program's input (`stdin: "{{file}}"`) and run only when the database doesn't exist yet (`if_missing: true`).

### Values that depend on the version: `vars` with `if`

`vars` (see the sources section) also work while the service runs, as `{{var.<name>}}` in files, arguments and the environment. A case can have an `if:` with a setting (`setting.<name>`) or a version comparison (`version>=10.11`, `version<9`; compared by prefix, so `version<9` holds for 8.4.7):

```yaml
vars:
  bind:
    - { value: "{{loopback}}", if: "version>=10.11" }   # 127.0.0.1,::1 where the machine has IPv6
    - { value: 127.0.0.1 }
```

`{{loopback}}` is `127.0.0.1,::1` where the machine has IPv6 loopback, else `127.0.0.1`. `{{log}}` is the service's own log file (its `log`), which the Logs page reads.

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

`run.test` gives arguments that check the config without starting (`httpd -t`, `caddy validate`); when it fails, the start stops with the program's own message.

### Command-line tools: `kind: cli` and `commands`

Not everything runs. `kind: cli` describes programs used from the terminal (Bun, Deno, uv): LampRabit downloads, checks and installs them like a service, and puts their `commands` on its PATH (its shims) for the active version. A `kind: cli` manifest has no `run`, `stop`, `ports`, `health`, `connection`, `ui` or `open`.

```yaml
schema: 1
id: bun
kind: cli
name: Bun
category: packages
blurb: JavaScript runtime and packages
source:
  github: oven-sh/bun
  versions: newest
  prefix: bun-
  assets:
    windows: { file: bun-windows-x64.zip, archive: zip }
    linux-x64: { file: bun-linux-x64.zip, archive: zip }
    macos-arm64: { file: bun-darwin-aarch64.zip, archive: zip }
binary: "bun{{exe}}"
commands:
  - { name: bun, program: "{{home}}/bun{{exe}}" }
  - { name: bunx, program: "{{home}}/bun{{exe}}", args: [x] }
```

| Field | What it is |
|---|---|
| `kind` | `service` (the default), `cli`, or `tool`. `tool` is LampRabit's own web tools served from their files (Adminer); manifests from a file can't use it. |
| `commands` | `name` (what's typed in the terminal), `program` (inside the install: `{{home}}/…`), `args` put before the typed ones, and `env`. The review before adding a file lists them. |

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
  - { runtime: python, min: "3.10", max: "3.14", prefer: "3.13", why: pgAdmin runs in a Python environment }
  # Tomcat 10 runs on Java 11, Tomcat 11 and later on Java 17:
  - { runtime: java, min: "11", since: "10", until: "11", why: Tomcat runs on the active Java }
  - { runtime: java, min: "17", since: "11", why: Tomcat runs on the active Java }
system_needs:
  - { command: erl, on: [unix], hint: "Install Erlang with your system's packages (sudo apt install erlang-nox), then start RabbitMQ again." }
```

`needs` names a runtime LampRabit installs: `java`, `erlang`, `python`, `postgres`, or `node` (NVM's). The install dialog installs it first (picking `prefer` when several versions fit), and the service won't start until it's installed in a version from `min` up to `max` (by prefix: `3.14` takes 3.14.x); the error says what to install. `since` and `until` limit a need to some of the service's own versions (from `since` on, below `until`), for requirements that change between releases; while listing, before a version is chosen, only needs without `until` count. `{{needs.<id>.home}}` and `{{needs.<id>.bin}}` point at it. `system_needs` looks for a command on PATH that LampRabit can't provide and shows `hint` when it's missing.

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

### A database client: `connects`

A database client (KarotX is one) can show every local database without the user typing anything. LampRabit keeps a services file in the client's data folder that lists every installed database service, running or stopped, and every SQLite file in the projects. It rewrites the file (atomically, readable only by the current user, since it holds passwords) within a second of a database being installed, removed, started or stopped. The client watches the file, so pass its path in `run`.

```yaml
run:
  args: [--services, "{{data}}/karotx-services.json"]
connects:
  file: "{{data}}/karotx-services.json"
  sqlite: sqlite                     # the client's type for SQLite files; leave out to skip them
  databases:                         # service id: the client's type for it
    mariadb: { type: mariadb }
    redis: { type: redis, database: "0" }
    clickhouse: { type: clickhouse, port: tcp, user: default }   # a port other than its main one
```

- Each entry gets `id` (the service id), `name` (service name and major version), `type`, `host` (127.0.0.1), `port`, `user`, `password` and `database` (from the service's `connection.url` unless set here), and `status` (`running` or `stopped`). The file's format is KarotX's: `{ "version": 1, "source": "LampRabit", "databases": [...] }`.
- `file` must be inside the client's data folder.

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
| `{{root}}` | LampRabit's folder |

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

LampRabit's own web servers and Tomcat name the **site engine** they're built around with `engine: apache | nginx | caddy | frankenphp | tomcat`. The engine is the one part that stays LampRabit's code: before every start it writes the virtual hosts, PHP's pools, the HTTPS certificates and the hosts file (Tomcat: a `<Host>` per Java project), and a web server's ports come from Settings → Web server (which one is primary). Their downloads, start, config test, stop and settings are in their manifests like any service's; `{{server.admin}}` is Caddy's or FrankenPHP's admin API port. Every other service, built in or not, is described entirely by its manifest. They may also use `source: { lamprabit: true }` for LampRabit's own CI builds, `checksum: none` (Typesense publishes no checksums). Community manifests can't use these.

## Checks

Every manifest is checked when it's read, and a broken one is refused with the reason: unknown fields (usually a typo), unknown placeholders or platforms, a port listed twice, a health check that names a missing port, paths that leave the service's folders, programs outside the install. The built-in manifests are checked by `cargo test` (`manifest::tests`), so a broken one never ships.
