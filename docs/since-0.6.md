# What's new since 0.6.0

Everything added, changed and fixed on the way to the next release, written for the website and the release notes. 0.6.0 is the latest release (2026-11-14). Add a line here with every feature or fix; when the release ships, this becomes its CHANGELOG.md section and the file starts over.

## New

### Services you can add yourself
- **Every service is a YAML file** (a manifest): its downloads, ports, settings, health check, backups, config files and docs. The 24 built-in services (Apache, Nginx, Caddy, FrankenPHP, Tomcat, MariaDB, MySQL, PostgreSQL, MongoDB, Redis, Memcached, Mailpit, ClickHouse, Neo4j, Meilisearch, Typesense, MinIO, RustFS, Elasticsearch, OpenSearch, RabbitMQ, Ollama, n8n, pgAdmin) all run from theirs.
- **Add a service from a file**: Services → Add a service, pick its `service.yaml`, review what it does, approve. It's kept across updates and moves with the LampRabit folder.
- `lamprabit service check <file>` reviews a service file offline; `lamprabit service try <file>` installs, starts, health-checks and stops it in a throwaway folder.
- Each service's page lists its **config files** (open, reveal, copy path) and its **documentation**; the Services search finds a service by other words ("db", "queue", "s3").
- **Settings per service** from its file, on its Services page.
- **Database backups** for MariaDB, MySQL, PostgreSQL and MongoDB come from the service files too.

### KarotX, LampRabit's own database client
- **KarotX** installs from Services and opens every local database: MariaDB, MySQL, PostgreSQL, MongoDB, Redis, ClickHouse, Neo4j and the SQLite files in your projects, running or not.
- It's the default database app; pick another per database (phpMyAdmin, Adminer, pgAdmin…) on the database's Services page or in Settings → Databases.

### Command palette
- **Ctrl+K / Cmd+K** (or Search in the title bar) finds every action, project, service, setting, config file and docs page: start a project's dev server, turn on a PHP extension, jump to a setting, open php.ini.
- Deep links and a project's tabs, logs and dev server are in it too.

### Home
- A **status strip** (services, projects, memory with each service's share) and **Needs attention**: what's wrong, with a one-click fix (free a port, start what a service needs, put NVM first in PATH).
- **Three layouts for the services**: rows, big tiles (three to a row) or compact tiles (four to a row), from the button next to Profile.
- **Running services stand out**: their icon and name turn green. Compact tiles show the status, port, live CPU and memory (or the version), Start/Stop with their labels, and a running service's name opens it.
- **Menus stay solid**: a ⋮ menu is no longer faded by a dimmed card behind it, and its labels don't wrap.
- **Stack profiles in one row**: pick a profile from a pill (its service count, a dot while it runs, ⋮ to edit or delete); the header's Start starts it.
- Tick services to **save them as a profile**, or to **update the active profile**.

### Projects
- **Laravel's own dev server**: Start dev server runs `composer run dev` (serve, queue, logs and Vite together) or `php artisan serve`, on a port LampRabit picks. Its arrow lists the ready-made commands (and the Vite script) and links to the project's settings.
- **New project, more choices**: Laravel's starter kits (React, Vue, Livewire) and Pest; Symfony as a web app or a bare skeleton; Django with Django REST framework; Flask with Flask-SQLAlchemy; Spring Boot's Initializr dependencies (JPA, security, actuator, database drivers…); a Spring Boot template with Gradle.
- **Python projects** choose venv + pip, uv or conda; Laravel's frontend step uses the Node package manager you pick.
- **Git: pull from or push to any branch** (⋯ next to Pull and Push), not only the tracked one.
- **Git: a Tags view**: tag any commit or branch, filter, see which tags are only local, push one or all, delete here or on the remote, fetch the remote's tags, start a branch from a tag.
- **Git: a Stashes view**: stash with a name (everything, tracked files only, or only what's staged), look inside any stash file by file, restore it, apply and keep it, turn it into a branch, delete one or clear them all.
- **PostgreSQL extensions** on PostgreSQL's Services page: per database, switch pgcrypto, pg_trgm, citext, hstore, unaccent and the rest on or off, and update them when the server has a newer version.
- **Templates from the CLI and AI assistants**: `lamprabit template show|example|from|save`, and the MCP tools `get_template`, `template_format`, `draft_template_from_project` and `save_template`, so you can ask your assistant to turn a project's setup into a LampRabit template.
- **Cloudflare Tunnel on your own domain**, after signing in to Cloudflare from LampRabit.

### Command line and MCP
- `lamprabit files`, `restart`, `terminal [project]`, `project edit`, `php extensions`, `php enable|disable <extension>`.
- MCP tools: config files, PHP extensions (list, turn on or off), restart the running services, open a project in the editor, and the template tools above.

## Changed
- **A quieter look**: neutral icons with colour only for status, sentence-case labels, no greeting or quote on Home.
- **Each page has its own header** with its title and actions; the title bar keeps only what's the same everywhere (search, theme, Open www, terminal, Start all), so it no longer crowds.
- **Links open what they need**: opening karotx.test, pgadmin.test, phpMyAdmin, a service's console or a project starts the web server and the service behind the link (and the database a client shows) first, instead of landing on "service unavailable".
- **Install buttons go to that service's page** (Install MySQL opens MySQL, Install KarotX opens KarotX…), not the top of Services.
- Services with a web console (ClickHouse, MinIO, RustFS, RabbitMQ, Meilisearch) and Mailpit open at their own `.test` address.
- A service shows when something it needs is stopped (pgAdmin without PostgreSQL), with Start.
- The app refreshes everywhere after any change, and when you come back to the window.

## Fixed
- **"Node.js isn't installed" inside LampRabit while every terminal finds node**: project commands and dev servers used the PATH LampRabit started with, so a Node (or NVM) installed afterwards was missing. They now use PATH as a new terminal sees it, and a Node from Volta, fnm, the nodejs.org installer or nvm set up in your shell profile works too.
- NVM for Windows installed into another folder than NVM_HOME (its `root:` setting) is found.
- Python projects' package manager choice didn't show in New project.
- Freeing a port held by an administrator's process (or by Windows' http.sys on port 80) asks for admin rights instead of failing.
- A port error stayed on a service after its port was changed.
- A service's tile says which port is in use and offers Stop and Change port.
- phpMyAdmin answered "unavailable" right after installing until a project was created.
- pgAdmin 9.19 is skipped: its download is published without its JavaScript, so it never loads. Install 9.18 from Services.
- MySQL backups couldn't be restored (the dump's GTID lines).
- Other stacks on Windows: a port held by another stack's Windows service can be freed; LampRabit's commands can go first in the system PATH (XAMPP's PHP won).
- Settings → Troubleshooting can exclude the LampRabit folder and Composer's cache from Windows Security scans, which stalled Composer.
- Home and Settings no longer crash on a service added from a file.
