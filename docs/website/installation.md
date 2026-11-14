# Installing LampRabit

> **Status:** installers for Windows, macOS (Apple silicon and Intel) and Linux are on the GitHub Releases page: https://github.com/LampRabit/LampRabit/releases/latest. All three are supported. What can be installed differs a little by OS and processor; the table below lists it, and the in-app Services catalog says why when something isn't available.

## System requirements

| | Windows | macOS | Linux |
|---|---|---|---|
| OS version | Windows 10 (1809 or later) or Windows 11, 64-bit | macOS 11 Big Sur or later (with Safari 16.4 or later); Apple silicon or Intel | Ubuntu 22.04+, Debian 12+, Fedora 38+ or similar, x86_64 or arm64 |
| Web view | Microsoft Edge WebView2 (built into Windows 11; the installer adds it on Windows 10) | Built in | WebKitGTK 4.1 |
| Runtime libraries | Microsoft Visual C++ 2015–2022 Redistributable (x64), needed by Apache and PHP; LampRabit offers to install it | — | MySQL needs libaio and libnuma (LampRabit names the packages) |
| Disk space | ~150 MB for the app and a basic stack; more for extra versions and databases | | |

### What can be installed on each platform

Every runtime is checksum-checked before use. "LampRabit's builds" are built from the official sources on GitHub Actions, because nobody publishes portable builds for that platform.

| Runtime | Windows | macOS | Linux |
|---|---|---|---|
| Apache | Yes (Apache Lounge) | Yes (LampRabit's builds) | Yes (LampRabit's builds) |
| Nginx | Yes (nginx.org) | Yes (LampRabit's builds) | Yes (LampRabit's builds) |
| Caddy | Yes | Yes | Yes |
| FrankenPHP | Yes | Apple silicon only | Yes |
| Tomcat 9 / 10.1 / 11 | Yes | Yes | Yes |
| PHP 8.1–8.5 | Yes (windows.php.net) | Yes (LampRabit's static builds) | Yes (LampRabit's static builds) |
| Node.js | Yes (through nvm-windows) | Yes (through nvm) | Yes (through nvm) |
| Java (Temurin), Python, Go, Rust | Yes | Yes | Yes |
| Composer, Bun, Deno, uv, conda, Maven, Gradle | Yes | Yes | Yes |
| MariaDB | Yes | Yes (LampRabit's builds of the LTS releases) | Yes |
| MySQL | Yes | Yes (Oracle's builds for recent macOS) | Yes |
| PostgreSQL 14–18 | Yes | Yes | Yes |
| MongoDB, mongosh, database tools, Compass | Yes | Yes | Yes |
| SQLite shell | Yes | Yes | Yes |
| ClickHouse | No (no Windows build; use WSL) | Yes | Yes |
| Neo4j | Yes | Yes | Yes |
| Redis (with redis-cli) | Yes (community redis-windows build) | Yes (LampRabit's builds) | Yes (LampRabit's builds) |
| Memcached | Yes (nono303's build, repackaged by LampRabit) | Yes (LampRabit's builds) | Yes (LampRabit's builds) |
| Meilisearch, Elasticsearch | Yes | Yes | Yes |
| Typesense | No (no Windows build) | Yes | Yes |
| OpenSearch | Yes | No (no macOS build) | Yes |
| RustFS | Yes | Apple silicon only | Yes |
| MinIO (last release, no longer maintained upstream) | Yes (LampRabit's build) | Yes (LampRabit's build) | Yes (LampRabit's build) |
| RabbitMQ | Yes (with Erlang) | Yes (needs your system's Erlang: `brew install erlang`) | Yes (needs your system's Erlang: `sudo apt install erlang-nox`) |
| Ollama, n8n | Yes | Yes | Yes |
| Git | Yes (MinGit), or your own Git | Your own Git (`xcode-select --install`) | Your own Git (`sudo apt install git`) |
| Mailpit, phpMyAdmin, Adminer, phpPgAdmin, pgAdmin, phpRedisAdmin | Yes | Yes | Yes |
| Cloudflare Tunnel, ngrok | Yes | Yes | Yes |

## Install

### Windows
1. Download `LampRabit_<version>_x64-setup.exe` (or the `.msi` for managed installs) from the Releases page.
2. Run it. LampRabit installs per user; administrator rights aren't required for the app itself.
3. **"Windows protected your PC"** appears because the installer isn't code-signed yet. Click **More info**, then **Run anyway**.
4. Apache and PHP need the Microsoft Visual C++ 2015–2022 Redistributable (x64). LampRabit checks for it and shows an **Install** button on the Overview page if it's missing.

### macOS
1. Download `LampRabit_<version>_aarch64.dmg` (Apple Silicon: M1 and later) or `LampRabit_<version>_x64.dmg` (Intel).
2. Open it and drag LampRabit to Applications.
3. Open LampRabit. Because it isn't signed yet, macOS says it can't be opened: go to **System Settings → Privacy & Security**, scroll down and click **Open Anyway**, then confirm. (Or right-click the app in Applications and choose **Open**.) You only need to do this once.

### Linux
- **AppImage:** download `LampRabit_<version>_amd64.AppImage`, make it executable (`chmod +x LampRabit_*.AppImage`) and run it.
- **Debian/Ubuntu:** `sudo apt install ./lampRabit_<version>_amd64.deb`

## First run

The setup guide walks you through it:

1. **LampRabit's folder.** Everything lives in one folder: `C:\lampRabit` on Windows, `~/lampRabit` on macOS and Linux, or wherever you choose (you can move it later in Settings → General):

   ```
   bin/      downloaded runtimes
   etc/      generated configuration
   www/      your projects: one folder per site
   data/     databases
   backups/  database backups
   logs/     all log files
   ssl/      local certificate authority and site certificates
   shims/    php, composer, mysql… for your terminal
   tmp/      sessions, uploads, temporary files
   ```

2. **What do you build?** Pick one or more (PHP, Laravel, WordPress, JavaScript, Python, Java, Go and Rust) and LampRabit installs what they need, one item at a time, with progress. You can add anything else from Services later.
3. **Import projects** from Laragon, XAMPP, MAMP, Herd and other tools, if you have them. They're copied into `www/`; the originals stay where they are.
4. **Create a first project**, or skip it.

Then:

- Click **Start all** in the title bar. (It leaves out heavy services such as Elasticsearch and Ollama; start those from their card or a stack profile.)
- When the web server starts, LampRabit adds your sites to the hosts file; your system asks for approval once, and again only when the list changes.
- On the Sites page, click **Trust certificate** and confirm. Restart your browser if it was open.
- Open `https://<project>.test`.

LampRabit's commands (`php`, `composer`, `mysql`…) are added to your PATH for new terminals (Settings → Terminal turns this off).

### Optional
- **Developer tools:** Services → Developer tools → install phpMyAdmin, Adminer and Mailpit (the setup guide offers them too).
- **No hosts file:** set the domain suffix to `localhost` in Settings. Sites become `hello.localhost` and browsers resolve them without the hosts file.

## Firefox and HTTPS

Firefox keeps its own certificate list. To make it trust LampRabit's certificates, open `about:config`, set `security.enterprise_roots.enabled` to `true`, and restart Firefox. Chrome, Edge, Brave and Safari use the system certificate store and work after **Trust certificate**.

On Linux, **Trust certificate** asks for your password once and adds the certificate to the system store. Chrome, Chromium and Firefox keep their own lists, which LampRabit updates too when `certutil` is installed (`sudo apt install libnss3-tools`, or `nss-tools` on Fedora).

On Linux, normal users can't listen on ports below 1024. The first time Apache or Nginx starts on port 80/443, LampRabit asks for your password once and saves the change in `/etc/sysctl.d/50-lamprabit.conf`. To avoid it, use ports above 1024 (such as 8080 and 8443) in Settings → Ports.

## Ports used

All of them can be changed in Settings → Ports (Mailpit's in Settings → Email testing); LampRabit warns when two would collide.

| Service | Default port |
|---|---|
| Primary web server (Apache, Nginx, Caddy or FrankenPHP) | 80, 443 |
| Other web servers next to it | Nginx 8080/8443, Apache 8081/8444, Caddy 8082/8445, FrankenPHP 8083/8446 |
| Tomcat | 8180 |
| PHP FastCGI workers | 9000 and up (10 per PHP version) |
| MariaDB / MySQL | 3306 / 3307 |
| PostgreSQL / pgAdmin | 5432 / 5050 |
| MongoDB | 27017 |
| ClickHouse | 8123 (HTTP), 19000 (native) |
| Neo4j | 7474 (Browser), 7687 (Bolt) |
| Redis / Memcached | 6379 / 11211 |
| Mailpit SMTP / inbox | 1025 / 8025 |
| Meilisearch / Typesense | 7700 / 8108 |
| Elasticsearch / OpenSearch | 9200 / 9201 |
| RustFS / MinIO | 9100 / 9110 (consoles one port above) |
| RabbitMQ | 5672, console 15672 |
| Ollama / n8n | 11434 / 5678 |
| Dump catcher / MCP server | 9912 / 39813 (127.0.0.1 only, when switched on) |

## npm and HTTPS behind a company proxy

If `npm install` fails with `UNABLE_TO_VERIFY_LEAF_SIGNATURE` or `SELF_SIGNED_CERT_IN_CHAIN`, something on your network (a company proxy, VPN or antivirus) re-signs HTTPS. LampRabit tells Node.js to trust your system's certificates, which works with Node.js 22.19 / 24.6 or newer: install a newer Node on Services → Node.js. Otherwise ask your IT team for the proxy's certificate and run `npm config set cafile <file>`.

## Uninstall

Uninstalling removes everything LampRabit set up and **keeps your projects (`www/`), databases (`data/`) and backups (`backups/`)**, so a later install picks them up again. It deletes the runtimes, configs, logs and certificates in the environment folder (`bin`, `etc`, `logs`, `tmp`, `ssl`, `shims`; nothing else in that folder is touched), LampRabit's settings, the `.test` entries in the hosts file, the trusted "lampRabit Local CA" certificate, LampRabit's commands on your PATH, the saved ngrok token and starting with your computer.

- **Windows:** Settings → Apps → LampRabit → Uninstall. The uninstaller asks you to close LampRabit, then does the cleanup; Windows asks for confirmation for the hosts file and the certificate. (Updating LampRabit never runs this.)
- **macOS and Linux:** in LampRabit, open Settings → Troubleshooting → **Remove LampRabit from this computer**. It shows what is deleted and kept before doing anything, then quits. Afterwards move LampRabit to the Trash (macOS), or delete the AppImage / run `sudo apt remove lamprabit` (Linux). On Linux your password is asked once, for the hosts file, the certificate and the ports 80/443 setting.
- The same Settings action works on Windows too, if you'd rather clean up before uninstalling.

To delete your projects and databases as well, delete the environment folder (`C:\lampRabit` or `~/lampRabit`) afterwards. Back up `www/` and `data/` first if you might need them.

If something couldn't be undone (for example a cancelled administrator prompt), LampRabit lists it with what to do by hand.

## Building from source

For contributors and early testers.

1. Install [Node.js](https://nodejs.org) 20 or later and [Rust](https://rustup.rs) (stable).
2. Platform prerequisites for Tauri:
   - Windows: Visual Studio 2022 Build Tools with the "Desktop development with C++" workload; WebView2 (built into Windows 11).
   - macOS: Xcode Command Line Tools (`xcode-select --install`).
   - Linux: WebKitGTK 4.1 and build tools, see https://tauri.app/start/prerequisites/
3. Clone and run:

   ```bash
   git clone https://github.com/LampRabit/LampRabit.git
   cd LampRabit
   npm install
   npm run tauri dev
   ```

4. Build an installer for your platform: `npm run tauri build`.
