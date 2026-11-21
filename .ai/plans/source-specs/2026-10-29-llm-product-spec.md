# LampRabit — Master Project Context & Product Evolution Specification

## 1. Project Overview

LampRabit is a modern, lightweight, cross-platform local development environment designed as an advanced alternative to XAMPP, Laragon, FlyEnv, and similar developer tools.

Its goal is to provide a unified environment where developers can create, configure, run, debug, manage, and deploy local projects without manually managing multiple runtimes, database servers, web servers, certificates, dependencies, and development tools.

LampRabit should evolve from a local PHP development utility into a comprehensive development workstation for PHP, Node.js, and other supported technology stacks.

### Core product vision

- Lightweight and fast, with a polished native desktop experience.
- Native operating-system processes for core services; Docker or WSL must not be mandatory.
- Multiple isolated projects with independent runtimes, domains, databases, and configurations.
- First-class support for PHP and Node.js, with an extensible architecture for additional technologies.
- Advanced project management, Git workflows, logs, diagnostics, and dependency management.
- AI-native workflows through a CLI, MCP server, and structured project information.
- A consistent experience across Windows, macOS, and Linux.

**Important:** LampRabit is an existing, actively developed project. Do not treat this specification as permission to rebuild it from scratch. Inspect the repository, identify what already exists, understand the current architecture, and integrate improvements without breaking working functionality.

---

## 2. Instructions for Working on the Existing Codebase

Before implementing anything:

1. Inspect the repository structure, package configuration, build system, application entry points, service managers, database integrations, UI components, and existing documentation.
2. Identify the current architecture, programming languages, frameworks, libraries, and operating-system-specific code.
3. Create an inventory of implemented, partially implemented, and missing features.
4. Identify existing conventions, abstractions, state management, persistence mechanisms, IPC boundaries, and security controls.
5. Reuse existing components and services wherever practical. Avoid duplicate implementations.
6. Preserve existing user data, configurations, projects, databases, and backward compatibility unless a deliberate migration is required.
7. Do not replace a working implementation with a new architecture solely because another approach seems cleaner.
8. Before modifying foundational systems, assess the impact on existing functionality and identify necessary migrations.
9. Keep documentation, tests, types, configuration schemas, and error handling synchronized with code changes.
10. Do not mark a feature as complete until it has been implemented, integrated, and appropriately tested.

When requirements are ambiguous, document the ambiguity and propose a safe default instead of making destructive assumptions.

Work incrementally. Separate discovery, architecture decisions, implementation, testing, and verification.

---

## 3. Core Architectural Principles

### 3.1 Native process management

LampRabit should manage services as native operating-system processes wherever feasible.

- No mandatory Docker or WSL dependency for core services.
- Support starting, stopping, restarting, monitoring, and terminating managed processes.
- Track process IDs, exit codes, health, ports, startup failures, and runtime status.
- Capture stdout and stderr with timestamps.
- Support graceful shutdown and platform-specific termination fallbacks.
- Detect stale processes, port conflicts, missing binaries, and orphaned services.
- Keep service lifecycle management consistent across operating systems.

Use containers only as an optional integration where a service cannot reasonably be supported natively or the user explicitly requests containerized execution.

### 3.2 Separate serving from routing

Application execution and domain routing must be independent systems.

**Serving layer**

Responsible for running the actual application, including PHP workers or web servers, Node.js development servers, and other language runtimes.

**Routing layer**

Responsible for mapping one or more local domains to the appropriate application endpoint.

Example:

- Project: `myproject`
- Domains: `myproject.test`, `api.myproject.test`
- Application server: `127.0.0.1:3000`
- Reverse proxy: Caddy, Nginx, Apache, or a supported built-in alternative
- HTTPS: locally trusted certificates, preferably through mkcert where appropriate

Support automatic port detection, configurable start commands, domain mapping, TLS configuration, and conflict detection.

The architecture must not assume that every project is served directly by Apache or Nginx.

### 3.3 Multiple processes per project

A project may require multiple concurrent processes.

Example:

- Web: `npm run dev`
- Queue worker: `php artisan queue:work`
- Scheduler: a framework-specific scheduler command
- Additional worker: `node worker.js`

Group these processes into named startup profiles.

Requirements:

- Start and stop a complete profile.
- Support individual process lifecycle controls.
- Display process-specific logs with distinct visual identification.
- Detect unhealthy or unexpectedly terminated processes.
- Allow per-process environment variables, working directories, ports, and commands.
- Support restart policies where appropriate.

### 3.4 Modular services and integrated core functionality

Use a consistent service registry and installation mechanism for optional runtimes and supporting services.

However, distinguish between external service binaries and LampRabit's own management capabilities.

The custom database management engine, project manager, service orchestration, configuration management, and core diagnostics must remain first-class integrated functionality rather than being fragmented into unrelated external applications.

---

## 4. Multi-Project and Multi-Virtual-Host Management

Implement comprehensive support for multiple virtual hosts within each individual project.

A project must not be restricted to a single domain.

Requirements:

- Associate multiple domains and subdomains with one project.
- Support separate document roots or route targets where needed.
- Map different domains to different ports or processes belonging to the same project.
- Support PHP applications, Node.js applications, and other supported runtimes.
- Configure HTTPS independently for each domain.
- Detect duplicate domains, port conflicts, invalid document roots, and conflicting proxy rules.
- Generate and update reverse-proxy configuration safely.
- Keep domain routing consistent when a project is moved, cloned, renamed, or reconfigured.
- Support wildcard domains where the operating system and DNS configuration permit them.
- Provide an understandable UI for managing domains, ports, TLS, and serving configuration.

Example:

A single project named `shop-platform` may expose:

- `shop.test` → storefront process
- `admin.shop.test` → admin application
- `api.shop.test` → API process
- `webhooks.shop.test` → webhook service

These domains may share a project directory while using different entry points, processes, or ports.

---

## 5. Reverse Proxy and Web Server Support

Support the following serving technologies:

- Apache
- Nginx
- Caddy
- FrankenPHP

Caddy and FrankenPHP must be treated as first-class options, not merely mentioned in documentation.

### Caddy

- Install and manage supported versions.
- Generate configuration for project domains.
- Configure local HTTPS and reverse proxy routing.
- Support Node.js and other port-based applications.
- Handle configuration validation and reloads safely.

### FrankenPHP

Provide seamless native integration.

- Install and manage FrankenPHP versions.
- Configure supported PHP application serving modes.
- Support its appropriate PHP execution models, including worker mode where applicable.
- Integrate virtual hosts, HTTPS, project environment settings, logs, and process management.
- Detect incompatible configurations rather than silently falling back to another server.
- Keep FrankenPHP-specific configuration separate from generic routing configuration where necessary.

### Apache and Nginx

Retain existing functionality while extending support for multi-domain projects and mixed application stacks.

Avoid maintaining several independent routing implementations if a shared domain and proxy configuration model can serve them.

---

## 6. PHP Runtime and Framework Management

PHP remains a core technology.

Requirements:

- Install and manage multiple PHP versions.
- Associate PHP versions and extensions with individual projects.
- Support per-project configuration such as `php.ini`, environment variables, and execution settings.
- Detect missing or incompatible extensions.
- Support Composer and framework-specific CLI commands.
- Manage PHP-FPM or the appropriate execution model for the selected server.
- Provide clear version and extension diagnostics.
- Avoid modifying global system PHP installations unexpectedly.

### Automated framework installers

Provide framework-specific project creation workflows for:

- Laravel
- Symfony
- CakePHP
- CodeIgniter
- Drupal
- Magento
- Statamic

Each installer should provide appropriate configuration, dependency installation, initial setup commands, database prompts, and local domain registration.

Framework installers must use supported upstream installation mechanisms and must not assume every framework uses the same directory structure or setup procedure.

For Magento and other resource-intensive frameworks, validate system requirements before starting installation.

---

## 7. Node.js and Other Runtime Support

Expand Node.js support into a first-class project runtime.

Requirements:

- Multiple Node.js versions and per-project version selection.
- Support project-specific package managers and lockfiles.
- Node.js projects served through local virtual hosts and HTTPS.
- Automatic or configurable port detection.
- Support standard Node.js execution, nodemon, and framework-specific development commands.
- Clone existing Node.js projects.
- Provide project templates for common frameworks.
- Support long-running processes, workers, background jobs, and schedulers.
- Configure environment variables and startup commands per project.
- Capture runtime logs and expose process health.
- Allow independent execution and routing configuration.

Support common Node.js frameworks through templates and configurable commands instead of hard-coding assumptions about every framework.

The broader architecture should accommodate Python, Go, Java, Ruby, Rust, and other runtimes through a consistent process and routing abstraction.

---

## 8. Database and Supporting Service Management

LampRabit should provide a unified service manager for databases, caches, queues, search engines, and other local development services.

### Database support

**MySQL and MariaDB**
- Preserve existing support.
- Manage multiple supported server versions where feasible.
- Provide startup, shutdown, configuration, logs, and connection details.

**PostgreSQL**
- Dedicated server installation and lifecycle management.
- Multiple versions and version-aware configuration.
- Database and user management.
- Connection information, logs, backups, and restore workflows.
- Support common framework integration requirements.

**MongoDB**
- Install and manage supported versions through appropriate official distribution binaries.
- Provide lifecycle controls and connection configuration.
- Support a dedicated database viewer.
- Validate platform and architecture compatibility.

**SQLite**
- Native file-based database support.
- Offer database file creation or selection during project setup.
- Prompt for SQLite integration when creating a new project or installing a framework that supports it.
- Provide an integrated SQLite viewer.
- Support database file inspection and appropriate backup workflows.

**Elasticsearch**
- Add optional installation and management support for search-oriented applications.
- Manage service health, configuration, ports, and logs.
- Keep its resource requirements visible to users.

### Cache and other services

Preserve existing Redis-compatible caching and Memcached functionality.

Where compatible with the current implementation:

- Use a suitable Redis-compatible server and CLI strategy for each platform.
- Prefer reliable native binaries over fragile compatibility layers.
- Do not assume that Redis and Memcached are interchangeable.
- Clearly distinguish PHP extensions from their corresponding server processes.

Potential additional services include:

- RabbitMQ
- Mailpit
- MinIO
- Meilisearch or Typesense
- n8n
- Ollama
- ClickHouse
- Neo4j
- Qdrant
- Stripe API mocking tools

These services should be introduced according to demand, platform feasibility, and resource requirements rather than bundled indiscriminately.

### Installation and version management

For every managed service, define:

- Supported operating systems and architectures.
- Available versions.
- Download source and integrity verification.
- Installation and upgrade behavior.
- Runtime configuration and data locations.
- Start, stop, restart, and health-check behavior.
- Backup and uninstall behavior.
- Known compatibility limitations.

---

## 9. Custom Database Management Engine

Build an integrated database management engine within LampRabit.

It must not depend on launching a separate external application as the primary database management experience.

The engine should provide a consistent interface for supported database types while allowing database-specific capabilities where needed.

Potential capabilities:

- Browse databases, schemas, tables, and collections.
- Inspect columns, indexes, relationships, and metadata.
- Execute SQL queries where supported.
- Display query results with pagination.
- Edit records where safe and supported.
- Create, modify, and remove database objects with appropriate safeguards.
- Import and export supported data formats.
- Manage connections, credentials, and database users where supported.
- View database server status and logs.
- Support backups, restores, and connection diagnostics.

Database-specific operations must respect the semantics and permissions of each database engine.

Provide dedicated integrated viewers or management interfaces for:

- PostgreSQL
- SQLite
- MongoDB
- Redis-compatible services

phpMyAdmin and phpRedisAdmin may remain supported integrations. phpPgAdmin may be provided where appropriate, but the integrated database engine remains the primary long-term management interface.

---

## 10. Onboarding, Installation Profiles, and Service Selection

Create a polished first-run onboarding experience.

The onboarding screen should help users select the tools and services they need without requiring every component to be installed.

Requirements:

- Explain LampRabit's core capabilities.
- Allow users to choose runtimes, web servers, databases, caches, and supporting tools.
- Show installation size, version, requirements, and any relevant platform limitations.
- Install selected services with progress, logs, cancellation where safe, and clear error recovery.
- Allow users to skip optional services.
- Provide sensible starter profiles such as PHP, Laravel, WordPress, Node.js, and full-stack development.
- Allow users to save custom profiles and reuse them on future setups.
- Support importing and exporting installation profiles.
- Allow users to install additional services after onboarding.
- Detect existing installations where possible and avoid unnecessary duplicate installations.

Profiles should describe desired service configurations rather than assuming every profile must install an entirely independent copy of every dependency.

---

## 11. Project Dashboard and Management

Introduce an optional, intuitive project dashboard for enhanced project management.

The dashboard should be useful without becoming cluttered or forcing every user into a dashboard-centric workflow.

Possible features:

- Project overview and status.
- Runtime and framework information.
- Domains, ports, and HTTPS status.
- Active processes and startup profiles.
- Database and service connections.
- Dependency status.
- Recent logs and errors.
- Git branch, working-tree status, and recent commits.
- Project health indicators.
- Quick actions for opening the project, terminal, browser, logs, or configuration.

Allow users to enable or disable the dashboard and customize which information is displayed.

### Modern UI/UX redesign

Redesign both the interface and the overall interaction model.

Design principles:

- Clear visual hierarchy.
- Reduced card density and unnecessary metadata.
- Strong distinction between primary actions and secondary information.
- Consistent spacing, typography, icons, colors, and component behavior.
- Responsive layouts within the desktop application's constraints.
- Useful empty states, loading states, errors, and success feedback.
- Keyboard navigation and accessibility.
- Clear service and project status indicators.
- Configurable workspace layouts.

### Docking and draggable tabs

Implement a flexible workspace with draggable tabs and dockable panels.

Requirements:

- Drag tabs between supported workspace locations.
- Dock panels to the left, right, top, or bottom edges of the workspace.
- Provide clear visual previews before completing a docking operation.
- Support tab groups and panel rearrangement.
- Preserve layout preferences across restarts.
- Prevent panels from being lost off-screen.
- Provide a reset-layout action.

"Dock anywhere" means flexible placement within supported workspace regions, with docking constrained to valid edges and drop targets rather than unrestricted floating windows unless floating panels are deliberately supported.

---

## 12. Advanced Logging and Diagnostics

Implement a project-based log capture system.

Logs should be associated with the correct project, process, service, and time range.

Requirements:

- Capture application stdout and stderr.
- Capture relevant web-server, PHP, database, and managed-service logs.
- Support multiple log streams per project.
- Filter by severity, source, process, date, and keyword.
- Provide search, sorting, timestamps, and contextual details.
- Allow users to clear, export, or inspect logs according to retention policies.
- Support live streaming and sensible log buffering.
- Avoid blocking the UI when logs are large.
- Support configurable retention and disk-usage limits.
- Highlight actionable errors without treating every warning as a fatal issue.

### Dump and dump catcher

Add a dedicated workflow for database dumps and relevant development payloads.

The exact meaning and scope of the existing "DD and dump catcher" note must be confirmed against the codebase and intended behavior before implementing it.

The eventual feature should clearly distinguish database dumps, import/export operations, and any request or payload capture functionality that may be intended.

---

## 13. Project Health Checker and Doctor

Provide a project health checker and a CLI/UI-accessible doctor command.

Its purpose is to identify and, where safe, resolve project environment problems.

Check for:

- Missing or incompatible runtime versions.
- Missing PHP extensions.
- Broken Composer or Node.js dependencies.
- Missing environment files or invalid configuration.
- Port conflicts and unavailable domains.
- Invalid or missing TLS certificates.
- Incorrect virtual-host and reverse-proxy configuration.
- Database connectivity and migration issues.
- Unavailable services.
- Git configuration problems.
- Framework-specific requirements.
- Incorrect file permissions and path configuration.
- Incompatible lockfiles or package-manager versions.

For each issue, show:

1. What is wrong.
2. Why it matters.
3. Which project or service is affected.
4. The recommended fix.
5. Whether the fix can be applied automatically.
6. What changes will be made before applying them.

Provide dry-run behavior and user confirmation for destructive or potentially disruptive repairs.

Never silently upgrade runtimes, delete dependencies, overwrite user configuration, or modify production resources.

---

## 14. Git and Version-Control Management

Provide first-class Git support for every project.

### Core Git functionality

- Initialize repositories.
- Clone projects.
- Display branch and working-tree status.
- Create, rename, switch, and delete branches.
- Stage, unstage, commit, and inspect changes.
- View commit history and diffs.
- Push, pull, fetch, merge, and handle conflicts.
- Configure repository-specific Git identity.
- Support custom usernames and email addresses for commits.
- Provide a dedicated Git configuration editor.

### Advanced Git workflows

- Commit splitting and related commit-management utilities.
- Branch comparison and change inspection.
- Repository cloning and remote management.
- GitHub, GitLab, Bitbucket, and other supported providers.
- Authentication through secure, supported credential mechanisms.
- Repository-specific and global settings with clear precedence.

Custom commit usernames and emails must affect the intended Git identity without unnecessarily modifying global user configuration.

### Git workflow builder

Build a configurable workflow system for common Git operations.

Examples:

- Create a feature branch.
- Run project checks.
- Commit changes.
- Push the branch.
- Open a pull or merge request where provider integrations permit it.

The workflow builder should support reusable steps, conditions, execution output, error handling, and explicit approval for consequential operations.

### Deployment checklist generator

Generate project-specific deployment checklists based on the project's runtime, framework, dependencies, database, Git status, and configured deployment requirements.

Include relevant checks such as:

- Dependency installation and lockfile validation.
- Build and test execution.
- Environment configuration.
- Database migrations and backups.
- Asset compilation.
- Secret and credential checks.
- HTTPS and domain configuration.
- Deployment verification and rollback preparation.

The checklist should be adaptable and must not assume every project deploys through the same hosting platform.

---

## 15. Dependency Manager

Create an integrated dependency management interface for Composer and Node.js ecosystems.

### Composer

- Inspect `composer.json` and `composer.lock`.
- Display installed dependencies and available project information.
- Run appropriate install, update, require, remove, and diagnostic commands.
- Display command output and errors.
- Detect PHP and extension compatibility problems.
- Preserve lockfile consistency.

### Node.js

- Support `package.json` and supported lockfile formats.
- Detect the project's package manager.
- Support install, update, add, remove, and diagnostic operations.
- Respect project-specific Node.js versions.
- Support npm, pnpm, Yarn, and other package managers where implemented.

Dependency operations should execute in the correct project directory and environment.

Do not automatically perform broad dependency upgrades without user consent.

---

## 16. Command-Line Interface (CLI)

Develop an official LampRabit CLI to make the environment accessible to developers, scripts, and AI coding assistants.

The CLI must be a supported interface to LampRabit's underlying management capabilities rather than a separate implementation with duplicated business logic.

Potential command groups:

- `lamprabit project`
- `lamprabit service`
- `lamprabit runtime`
- `lamprabit domain`
- `lamprabit database`
- `lamprabit logs`
- `lamprabit git`
- `lamprabit doctor`
- `lamprabit template`
- `lamprabit profile`
- `lamprabit config`
- `lamprabit mcp`

Illustrative workflows:

- List all projects.
- Create a project from a template.
- Register a project domain.
- Start or stop a project's services.
- Inspect service health.
- Query project logs.
- Run a project health check.
- Install a runtime or supporting service.
- Execute Git operations.
- Create or export a project profile.

The actual command syntax should follow the existing repository's conventions where applicable.

### CLI requirements

- Consistent argument parsing and help output.
- Machine-readable JSON output alongside human-readable output.
- Stable exit codes.
- Clear errors and actionable recommendations.
- Non-interactive execution for automation.
- Explicit confirmation or dry-run options for destructive actions.
- Secure handling of credentials and sensitive environment variables.
- Version information and compatibility checks.
- Documentation and tests.

---

## 17. MCP Server and AI Integration

Build a Model Context Protocol (MCP) server for LampRabit.

Its purpose is to allow AI coding assistants and other compatible tools to inspect and operate the developer's local environment through controlled, structured tools.

### Initial MCP capabilities

- List and inspect projects.
- Retrieve project runtime and framework information.
- Inspect registered domains and ports.
- Query service status and health.
- Read filtered project logs.
- Inspect dependency and configuration diagnostics.
- Run the project doctor.
- List available templates.
- Create projects from approved templates.
- Manage supported project and service lifecycle operations.
- Inspect Git status and recent changes.
- Query database connection metadata without exposing credentials.

### AI-native architecture

The CLI and MCP server should share the same core application services.

Prefer structured inputs and outputs, documented schemas, consistent error handling, and predictable behavior.

The MCP server should expose granular tools rather than giving an AI unrestricted access to arbitrary system operations.

Security requirements:

- Restrict operations to configured project and service boundaries.
- Require explicit approval for destructive actions.
- Avoid exposing credentials, secrets, or private keys.
- Provide safe read-only operations wherever possible.
- Apply appropriate authentication and transport security for remote access.
- Log important actions with suitable privacy controls.

Where practical, AI assistants should be able to use LampRabit to diagnose environment problems, inspect project state, create projects, and execute approved development workflows without needing to manually navigate the UI.

---

## 18. Declarative Project Templates and Recording

Build a robust project template engine.

### Declarative templates — primary system

Use a structured, versioned template format such as YAML or a suitable equivalent.

Templates should support:

- Template name and description.
- User-defined variables and default values.
- Input validation.
- Conditional steps.
- Sequential command execution.
- Runtime and service prerequisites.
- Dependency installation.
- Project configuration.
- Database setup.
- Domain registration.
- Git initialization.
- Live execution output.
- Failure handling and cleanup where safe.

Examples of templates:

- Laravel application.
- WordPress installation.
- CodeIgniter application.
- Node.js API.
- React frontend.
- Full-stack monorepo.
- PHP and PostgreSQL application.

### Record and convert — advanced feature

Allow developers to record project setup operations and convert them into reusable templates.

Potential workflow:

1. Start recording mode.
2. Capture approved terminal commands and supported setup actions.
3. Review the captured steps.
4. Remove sensitive commands or information.
5. Replace hard-coded values with variables.
6. Validate the resulting template.
7. Save, reuse, export, or share it.

Recording must not capture passwords, access tokens, or other secrets by default.

### Template history and CLI integration

Record which template was used to create a project, its version, and relevant execution results.

Support template discovery, execution, validation, and recording through the CLI.

Template history should help users reproduce project environments and troubleshoot setup failures without pretending to be a complete version-control system.

---

## 19. n8n and Additional Integrations

Include n8n as an optional managed service.

Requirements:

- Install and manage supported versions.
- Configure its runtime, ports, and environment.
- Integrate it with local domains and HTTPS.
- Capture logs and health status.
- Support start/stop/restart operations.
- Clearly identify its database and persistence requirements.
- Preserve workflow data when upgrading or uninstalling.

Additional integrations should follow the same service-management standards and should be introduced based on demand and platform compatibility.

---

## 20. Cross-Platform Compatibility

Extend full operational compatibility to:

- Windows
- macOS
- Linux

Cross-platform support must be validated through actual platform-specific implementation and testing, not merely through shared UI code.

Requirements:

- Platform-appropriate binary downloads.
- Architecture detection.
- Correct filesystem paths and permissions.
- Platform-specific process lifecycle management.
- Service installation and cleanup.
- Hosts-file or local DNS integration.
- Local TLS certificate generation and trust configuration.
- Correct handling of process signals and termination.
- Port inspection and conflict detection.
- Platform-specific limitations and fallback behavior.

Do not assume a Windows-specific executable, service mechanism, or filesystem operation will work on macOS or Linux.

Maintain a clear compatibility matrix for runtimes, servers, databases, extensions, and optional services.

---

## 21. Installer and Uninstaller Safety

Installation and removal must be reliable, predictable, and reversible where possible.

### Installation

- Download and verify required binaries.
- Track installed versions and managed files.
- Handle interrupted installations.
- Validate prerequisites.
- Avoid overwriting unmanaged installations.
- Provide understandable failure recovery.

### Uninstallation

The original requirement mentions removing the LampRabit folder during uninstall, but the exact exception to that rule needs clarification.

Use a safe default: distinguish application-managed installation files from user-owned projects, databases, profiles, logs, and configuration.

- Clearly show what will be removed.
- Preserve user data by default.
- Offer explicit options for deleting user data when appropriate.
- Avoid deleting arbitrary directories outside the managed installation.
- Clean up registered domains, certificates, services, and PATH entries only when they belong to LampRabit and removal is authorized.
- Ensure upgrades and reinstalls do not unintentionally erase user work.

---

## 22. Security, Reliability, and Resource Management

Because LampRabit manages local servers, files, processes, credentials, and databases, safety must be part of the core design.

- Validate user-provided paths and commands.
- Avoid shell-injection vulnerabilities.
- Restrict filesystem operations to approved locations where possible.
- Protect secrets and credentials.
- Use least-privilege operations for system changes.
- Warn users when services are exposed beyond localhost.
- Detect port conflicts before modifying routing configuration.
- Verify downloaded binaries and package integrity.
- Protect databases and project data during upgrades and removal.
- Avoid uncontrolled background processes.
- Provide recovery mechanisms for failed configuration changes.
- Keep resource-intensive services optional.

---

## 23. Recommended Implementation Priorities

Treat these as planning priorities, not permission to ignore the current implementation.

### Phase 1 — Audit and architectural foundation

- Inspect the existing repository.
- Establish the current feature inventory.
- Identify architectural gaps.
- Strengthen service and process management.
- Establish shared project, runtime, domain, and service configuration models.
- Implement reliable multi-virtual-host routing.
- Validate Caddy and FrankenPHP integration.
- Establish a cross-platform compatibility plan.

### Phase 2 — Core user experience

- Redesign the UI and UX.
- Improve the dashboard and project management.
- Implement flexible dockable panels and draggable tabs.
- Add onboarding, service selection, and saved profiles.
- Improve service installation and uninstallation safety.

### Phase 3 — Project runtime expansion

- Strengthen Node.js support.
- Add framework-specific project templates.
- Implement declarative templates and template history.
- Expand database support to PostgreSQL, MongoDB, and SQLite.
- Build the integrated database management engine.
- Add project health checks and diagnostics.

### Phase 4 — Advanced developer tools

- Implement project-based logging and search.
- Add Composer and Node.js dependency management.
- Expand Git operations and workflow automation.
- Build deployment checklist generation.
- Implement project cloning and configuration utilities.
- Clarify and implement the dump catcher requirement.

### Phase 5 — AI-native workflows

- Build the LampRabit CLI.
- Expose shared capabilities through MCP.
- Integrate project creation, template recording, and health diagnostics.
- Support controlled AI-driven development workflows.

### Phase 6 — Additional services and ecosystem

- Add n8n and selected optional services.
- Expand Elasticsearch integration.
- Add additional service binaries and version management.
- Improve platform coverage.
- Establish a community template and service registry where appropriate.

The order can change when the repository audit identifies dependencies, existing implementations, or more urgent stability issues.

---

## 24. Definition of Done

A feature is complete only when:

- It integrates with the existing architecture.
- Its configuration and persistence behavior are defined.
- It works with relevant existing projects and services.
- Errors are handled and displayed clearly.
- Appropriate tests cover expected behavior and failure scenarios.
- Platform-specific limitations are documented.
- User data is preserved.
- Security implications have been assessed.
- Documentation is updated.
- The implementation has been verified rather than merely scaffolded.

Do not introduce placeholder UI elements that appear functional but have no working backend behavior.

Do not claim support for an operating system, runtime, framework, or service without verifying the actual implementation.

---

## 25. Final Direction

LampRabit should become a polished, lightweight, AI-native local development platform that combines the simplicity of traditional local server tools with the flexibility of modern development environments.

The long-term product should offer:

- A unified manager for local development services.
- First-class PHP and Node.js workflows.
- Multiple domains and processes per project.
- Native reverse-proxy and HTTPS management.
- Integrated database and Git tooling.
- Declarative and recorded project templates.
- Project health diagnostics and dependency management.
- A CLI and MCP interface for AI-assisted workflows.
- Consistent operation across Windows, macOS, and Linux.

**Most important implementation rule:** Preserve the working application and build on top of its actual architecture. Use this specification to guide product evolution, not as a reason to rewrite functioning components unnecessarily.

Start by inspecting the current repository and mapping these requirements to existing functionality. Then identify the highest-value gaps, dependencies, and architectural risks before proposing or implementing the next set of changes.
