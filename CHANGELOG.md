# Changelog

What changed in each LampRabit release. Downloads are on the [Releases page](https://github.com/LampRabit/LampRabit/releases).

## 0.6.0

**macOS is supported.** It's no longer a preview: Apple silicon and Intel, checked by hand end to end (sites over HTTPS, databases, Mailpit, Apache and Nginx, the terminal and PATH). A few services have no build for macOS or Intel Macs; the [installation guide](docs/website/installation.md#what-can-be-installed-on-each-platform) now lists what can be installed on each OS and processor.

### New
- **Dependencies tab** in every project: each package manager's packages (Composer, npm, pnpm, Yarn, Bun, Deno, uv, Poetry, pip, Cargo, Go, Maven, Gradle) with what the project asks for and what's installed. Check for updates, then install, add, update, upgrade or remove; every change shows its command and runs only when you confirm.
- **Bun, Deno, uv and conda (Miniforge)** install from Services → Package managers, with `bun`, `bunx`, `deno`, `uv`, `uvx`, `conda` and `mamba` in your terminal. pnpm and Yarn turn on from the Node.js card (through corepack).
- **More project commands:** deno.json tasks, pyproject.toml scripts, requirements.txt, Maven phases and Gradle tasks (through the project's `mvnw`/`gradlew`), Cargo and Go.
- **npm behind company proxies:** everything LampRabit runs uses your system's certificates, so `npm install` works where a proxy, VPN or antivirus re-signs HTTPS (Node.js 22.19 / 24.6 or newer). When a certificate error still happens, the message explains it.

### Changed
- **Start all** (and the tray, start on launch and `lamprabit start`) leaves out the heavy services: Elasticsearch, OpenSearch, ClickHouse, Neo4j, Ollama, n8n and RabbitMQ. Start them from their card or a stack profile.
- **LampRabit's commands are on your PATH by default** on new installs (Settings → Terminal turns it off).
- **App updates stop your services cleanly** before installing, so databases shut down properly.
- macOS: the window buttons sit in the middle of the title bar.

### Faster
- The app opens with half the code it used to load; the other pages load in the background.
- Busy logs no longer slow the window: lines arrive in batches.
- Pages you aren't looking at stop refreshing, and process scans no longer hold up the window.

### Fixed and hardened
- The window now runs with a strict content security policy, and the tokens that guard the command line's and the MCP server's local ports come from the system's random source.
- macOS: updating `/etc/hosts` works when LampRabit's folder name contains quotes.
- The MCP server's port no longer appears in the main web server's settings.
- Error rows in Logs line up with the others.
- Updates and runtime downloads use the LampRabit organisation's address on GitHub (older addresses still work).
