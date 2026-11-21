# Phase 24: languages

Status: **done**. Listed in the [roadmap](../roadmap.md).

**Done (2026-11-01):** Python as a runtime; Python projects (`sites::Language::Python`, Django/Flask/FastAPI detected) and Java projects serve through their template's dev server like Node; templates Django, Flask, FastAPI, Spring Boot (Spring Initializr + Maven wrapper) and a plain Java web app for Tomcat; `requires.python`/`requires.java`; JAVA_HOME for steps and dev servers.
Then Go (go.dev, the two supported lines) and Rust (rustup-init into bin/rust with RUSTUP_HOME/CARGO_HOME there; Windows uses the GNU host so no Visual Studio is needed), `Language::Go`/`Rust` with Gin/Echo/Fiber/Chi and Axum/Actix/Rocket detection, templates Go (net/http), Gin and Axum. Per-project versions (`langver.rs`): `.python-version`, `.java-version`/`.sdkmanrc`, `go.mod`'s toolchain/go line and `rust-toolchain.toml` pick the installed version (exact, else newest with the same major.minor); the python/java/go shims choose by folder like PHP's, rewritten when a version file changes (watch loop); Sites shows the version and its file, with a picker that writes `.python-version`/`.java-version`. rustup handles Rust's file itself.

Python, Go, Rust and Java, each as a runtime with versions side by side, like PHP. Decided 2026-11-01: LampRabit installs its own copies under `bin/` (pyenv, sdkman and the like aren't used):
- Sources in the table above. Rust goes through rustup with `RUSTUP_HOME`/`CARGO_HOME` under `bin/rust`, so it stays inside LampRabit too.
- Per-project version from the usual file (`.python-version`, `go.mod`'s `toolchain`, `rust-toolchain.toml`, `.java-version`), shown and changeable in the project's Settings. Shims (`python`, `go`, `java`) pick it like the PHP shim.
- `sites::Language` gains `Python`, `Go`, `Rust` and `Java`. Serving needs nothing new, since port discovery routes any process by its working folder. `apps::suggest` learns `manage.py runserver`, `uvicorn`/`flask run`, `go run .`, `cargo run`, `mvn spring-boot:run`/`gradle bootRun`. WAR projects can deploy to Tomcat instead.
- Templates: Django, Flask, FastAPI, Go (net/http, Gin), Rust (Axum), Spring Boot (start.spring.io), Quarkus.
