//! Projects: creating them, templates, lamprabit.toml, rename and delete, facts and the doctor.

use crate::*;

/// Creates a project in www/ from a template and returns its URL. Progress is streamed as
/// `project-progress` events; a failed project is removed so it can be retried.
#[tauri::command]
pub(crate) async fn create_project(app: AppHandle, state: State<'_, AppState>, request: projects::NewProject) -> Result<CreatedProject> {
    let (_, layout) = state.snapshot().await;
    projects::validate(&request, &layout)?;
    let (template, source) = templates::find(&layout, &request.template)?;
    let name = request.name.trim().to_string();
    state
        .update_settings(|s| {
            let site = s.sites.entry(name.clone()).or_default();
            site.php = request.php.clone();
            site.server = request.server;
            site.ssl = Some(request.ssl);
            site.template = Some(format!("{}@{}", template.id, template.version));
        })
        .await?;
    let (settings, layout) = state.snapshot().await;
    let url = sites::url_for(&settings, &name).ok_or_else(|| Error::msg("invalid project name"))?;
    let ctx = projects::Ctx { app: &app, http: &state.http, settings: &settings, layout: &layout, name: &name, created_database: Default::default() };
    logs::system(&app, logs::Level::Info, format!("Creating project {name} from the {} template", template.name));

    let result = async {
        let creates_database = template.steps.iter().any(|s| matches!(s.action, templates::Action::CreateDatabase(_)));
        if request.database.is_some() && creates_database {
            use templates::DbEngine;
            let engine = projects::engine(&settings, &request).ok_or_else(|| Error::msg("Install MariaDB, MySQL or PostgreSQL first (Services)."))?;
            if !template.engines.contains(&engine) {
                let supported: Vec<&str> = template.engines.iter().map(|e| e.label()).collect();
                return Err(Error::msg(format!("{} projects can use {}, not {}.", template.name, supported.join(" or "), engine.label())));
            }
            let kind = match engine {
                DbEngine::Mariadb => ServiceKind::Mariadb,
                DbEngine::Mysql => ServiceKind::Mysql,
                DbEngine::Postgres => ServiceKind::Postgres,
                DbEngine::Mongodb => ServiceKind::Mongodb,
                // Nothing to start: PHP has SQLite built in.
                DbEngine::Sqlite => return projects::build(&ctx, &request, &template, source, &url).await,
            };
            if settings.versions.get(engine.id()).is_none() {
                return Err(Error::msg(format!("Install {} first (Services).", engine.label())));
            }
            if !state.supervisor.is_running(kind).await {
                ctx.step(format!("Starting {}", engine.label()));
                state.supervisor.start(&app, kind, &settings, &layout).await?;
            }
            // The process is up before it accepts connections.
            for _ in 0..30 {
                let probe = match engine {
                    DbEngine::Postgres => diagnose::probe_postgres(settings.ports.service("postgres.postgres")).await,
                    DbEngine::Mongodb | DbEngine::Sqlite => diagnose::Probe::Ok,
                    DbEngine::Mysql => diagnose::probe_mysql(settings.ports.service("mysql.mysql")).await,
                    DbEngine::Mariadb => diagnose::probe_mysql(settings.ports.service("mariadb.mysql")).await,
                };
                if matches!(probe, diagnose::Probe::Ok) {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
        projects::build(&ctx, &request, &template, source, &url).await
    }
    .await;

    let built = match result {
        Ok(built) => built,
        Err(e) => {
            let _ = std::fs::remove_dir_all(layout.www().join(&name));
            projects::drop_created_database(&ctx, &request).await;
            state.update_settings(|s| { s.sites.remove(&name); }).await?;
            logs::system(&app, logs::Level::Error, format!("Creating project {name} failed: {e}"));
            return Err(e);
        }
    };
    // A Node project gets its run setup, on a port of its own.
    if let Some(mut app_config) = built.app {
        app_config.port = apps::free_port(&*state.settings.read().await);
        state.update_settings(|s| s.sites.entry(name.clone()).or_default().app = Some(app_config)).await?;
    }
    // The template's processes ("auto" ports picked one by one, so they differ) and subdomains.
    if !built.processes.is_empty() || !built.domains.is_empty() {
        let mut scratch = state.settings.read().await.clone();
        let mut processes = built.processes;
        for i in 0..processes.len() {
            if processes[i].port == Some(0) {
                processes[i].port = Some(apps::free_port(&scratch));
            }
            scratch.sites.entry(name.clone()).or_default().processes = processes[..=i].to_vec();
        }
        let parent = format!("{}.{}", sites::label(&name).unwrap_or_default(), scratch.tld);
        let domains: Vec<config::ExtraDomain> = built.domains.into_iter().map(|(sub, target)| config::ExtraDomain { host: format!("{sub}.{parent}"), target }).collect();
        state
            .update_settings(|s| {
                let own = s.sites.entry(name.clone()).or_default();
                own.processes = processes;
                own.domains = domains;
            })
            .await?;
    }
    // Composer may have just been installed; this makes it the active version and adds its shim.
    state.update_settings(reconcile_versions).await?;
    ctx.step("Setting up HTTPS and the hosts file");
    prepare_new_site(&app, &state, &name).await;
    ctx.step("Adding the site to the web server");
    restart_web_servers(&app, &state).await?;
    logs::system(&app, logs::Level::Info, format!("Created project {name} at {url}"));
    Ok(CreatedProject { url, note: built.note })
}

#[derive(Serialize)]
pub(crate) struct CreatedProject {
    pub(crate) url: String,
    /// The template's "what next", e.g. finishing the WordPress installer.
    pub(crate) note: Option<String>,
}

/// Every project template: built-in, then the user's, with those that couldn't be read.
#[tauri::command]
pub(crate) async fn list_templates(state: State<'_, AppState>) -> Result<templates::Catalog> {
    let (_, layout) = state.snapshot().await;
    Ok(templates::catalog(&layout))
}

/// A template that recreates the project, as YAML for the template editor.
#[tauri::command]
pub(crate) async fn template_from_project(state: State<'_, AppState>, name: String) -> Result<String> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let site = sites::scan(&settings, &layout).into_iter().find(|s| s.name == name);
    let facts = facts::read(&dir, site.as_ref().and_then(|s| s.framework), "npm").await;
    let php = site.as_ref().filter(|s| s.language == sites::Language::Php).and_then(|s| s.php.clone().or_else(|| settings.versions.php.clone()));
    let has_database = site.as_ref().is_some_and(|s| s.database.is_some());
    Ok(templategen::generate(&templategen::Setup {
        name: &name,
        dir: &dir,
        remote: facts.git.and_then(|g| g.remote),
        php,
        engine: has_database.then(|| manage::engine(&settings, &dir)).flatten(),
        has_database,
        own: settings.sites.get(&name),
        parent: format!("{}.{}", sites::label(&name).unwrap_or_default(), settings.tld),
    }))
}

/// Saves a template's YAML into the templates folder after the same checks loading does.
/// An existing template with its id is replaced only when `replace` says so.
#[tauri::command]
pub(crate) async fn save_template(app: AppHandle, state: State<'_, AppState>, yaml: String, replace: bool) -> Result<String> {
    let (_, layout) = state.snapshot().await;
    let template = templates::parse_text(&yaml).map_err(|e| Error::msg(format!("The template isn't valid yet: {e}")))?;
    let dir = templates::folder(&layout);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.yaml", template.id));
    if path.exists() && !replace {
        return Err(Error::msg(format!("You already have a template with the id {}. Change the id, or replace it.", template.id)));
    }
    std::fs::write(&path, yaml)?;
    logs::system(&app, logs::Level::Info, format!("Saved template {} ({})", template.name, path.display()));
    Ok(template.id)
}

/// Opens the folder for your own templates, creating it with an example the first time.
#[tauri::command]
pub(crate) async fn open_templates_folder(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    use tauri_plugin_opener::OpenerExt;
    let (_, layout) = state.snapshot().await;
    let dir = templates::folder(&layout);
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("example.yaml.txt"), templates::EXAMPLE)?;
    }
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| Error::msg(e.to_string()))
}

/// The project doctor's issues for a project, most serious first (doctor.rs).
#[tauri::command]
pub(crate) async fn project_doctor(state: State<'_, AppState>, name: String) -> Result<Vec<doctor::Issue>> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let site = sites::scan(&settings, &layout).into_iter().find(|s| s.name == name).ok_or_else(|| Error::msg(format!("There's no project called {name} in www.")))?;
    let own = settings.sites.get(&name);
    let pm = own.and_then(|s| s.app.as_ref()).and_then(|a| a.package_manager).unwrap_or_else(|| apps::detect_package_manager(&dir));
    let facts = facts::read(&dir, site.framework, pm.id()).await;
    let database = if site.database.is_some() { database_status(state.clone(), name.clone()).await.ok().flatten() } else { None };
    let kind = ServiceKind::WEB.into_iter().find(|k| k.web_server() == Some(site.server)).unwrap_or(ServiceKind::Apache);
    let server_installed = settings.versions.get(kind.id()).is_some();
    let server_running = state.supervisor.is_running(kind).await;
    let info = list_sites(state.clone()).await?;
    let git_identity = if dir.join(".git").exists() {
        git::configure(&settings, &layout);
        git::status(&dir).await.ok().map(|s| s.identity.name.is_some() && s.identity.email.is_some())
    } else {
        None
    };
    let language = serde_json::to_value(site.language).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
    // The runtime besides PHP: Node from its settings or version file, the others from theirs.
    let node = (site.language == sites::Language::Node).then(|| {
        let chosen = own.and_then(|s| s.app.as_ref()).and_then(|a| a.node.clone());
        let file = [".nvmrc", ".node-version"].into_iter().find(|f| dir.join(f).is_file()).map_or("its .nvmrc", |f| if f == ".nvmrc" { "its .nvmrc" } else { "its .node-version" });
        let wanted = chosen.clone().map(|v| (v, "its settings")).or_else(|| node::project_version(&dir).map(|v| (v, file)));
        doctor::NodeInput {
            installed: node::installed_versions(),
            nvm: node::nvm_installed(),
            wanted,
            version: node::for_project(chosen.as_deref(), &dir).ok().map(|(v, _)| v),
            engines: facts.node_requirement.clone(),
        }
    });
    let runtime = match site.language {
        sites::Language::Python => Some(("Python", Runtime::Python)),
        sites::Language::Java => Some(("Java", Runtime::Java)),
        sites::Language::Go => Some(("Go", Runtime::Go)),
        // rustup installs the toolchain a project names by itself.
        _ => None,
    }
    .map(|(label, rt)| {
        let installed = runtimes::installed(&layout, rt);
        doctor::RuntimeInput {
            label,
            installed: !installed.is_empty(),
            matched: site.version_request.as_ref().is_some_and(|r| langver::matching(&installed, &r.version).is_some()),
            request: site.version_request.as_ref().map(|r| (r.version.clone(), r.file)),
            version: site.language_version.clone(),
        }
    });
    let app = if site.serve_mode == config::ServeMode::Dynamic && !matches!(site.language, sites::Language::Php | sites::Language::Static) {
        let status = state.apps.status(&name).await;
        Some(doctor::AppInput { state: status.state, exit_code: status.exit_code, routed: site.route.is_some() })
    } else {
        None
    };
    let input = doctor::Input {
        name: &name,
        dir: &dir,
        layout: &layout,
        language: &language,
        framework: site.framework,
        url: &site.url,
        ssl: site.ssl,
        php: site.php.as_deref().or(settings.versions.php.as_deref()),
        php_installed: runtimes::installed(&layout, Runtime::Php),
        php_requirement: facts.php_requirement.as_deref(),
        dependencies: facts.dependencies.iter().map(|d| (d.manager.clone(), d.installed, d.outdated)).collect(),
        package_manager: pm.id(),
        database,
        server: (kind.id(), kind.label(), server_installed, server_running),
        hosts_ok: info.hosts_ok,
        needs_apply: info.needs_apply,
        ca_trusted: info.ca_trusted,
        git_identity,
        node,
        runtime,
        app,
    };
    Ok(doctor::check(&input).await)
}

/// Applies one doctor fix, by the issue's id. The issues are worked out again first, so a
/// fix only runs while its problem is still there; each goes through the app's own command
/// for it, with that command's checks.
#[tauri::command]
pub(crate) async fn doctor_fix(app: AppHandle, state: State<'_, AppState>, name: String, id: String) -> Result<String> {
    use doctor::FixAction as A;
    let issues = project_doctor(state.clone(), name.clone()).await?;
    let issue = issues.into_iter().find(|i| i.id == id).ok_or_else(|| Error::msg("That problem is already gone (or changed): look at the list again."))?;
    let fix = issue.fix.ok_or_else(|| Error::msg("LampRabit can't fix this one by itself; the description says what to do."))?;
    logs::system(&app, logs::Level::Info, format!("Doctor, {name}: {}", fix.label));
    match fix.action {
        A::SetPhp { version } => set_site_php(app, state, name, Some(version)).await?,
        A::EnableExtension { php, name: ext } => {
            let (_, layout) = state.snapshot().await;
            let bundled = layout.bin("php", &php).join("ext").join(format!("php_{ext}.dll")).exists();
            if bundled {
                set_php_extension(app, state, php, ext, true).await?
            } else {
                install_php_extension(app, state, php, ext).await?
            }
        }
        A::RunCommand { label, command } => run_project_command(app, state, name, label, command).await?,
        A::CopyFile { from, to } => {
            let (_, layout) = state.snapshot().await;
            let dir = project_dir(&layout, &name)?;
            let target = dir.join(&to);
            if target.exists() {
                return Err(Error::msg(format!("{to} exists already; nothing was overwritten.")));
            }
            std::fs::copy(dir.join(&from), target)?;
        }
        A::StartService { service } => {
            let kind: ServiceKind = serde_json::from_value(serde_json::Value::String(service.clone())).map_err(|_| Error::msg(format!("Unknown service {service}")))?;
            start_service(app, state, kind).await?
        }
        A::CreateDatabase => create_project_database(app, state, name).await?,
        A::SwitchDatabase { engine } => switch_project_database(state, name, engine).await?,
        A::SetEnv { key, value } => set_project_env(state, name, key, value).await?,
        A::ApplySites => apply_sites(app, state).await?,
        A::TrustCa => {
            if let Some(note) = trust_ca(app, state).await? {
                return Ok(note);
            }
        }
        A::InstallNode { version } => node_install(app, version).await?,
        A::SetNode { version } => {
            // The project's saved run settings (or what it would start with) on that version.
            let (settings, layout) = state.snapshot().await;
            let dir = project_dir(&layout, &name)?;
            let mut config = settings.sites.get(&name).and_then(|s| s.app.clone()).unwrap_or_else(|| apps::suggest(&dir, 0));
            config.node = Some(version);
            app_save(app, state, name, config).await?
        }
        A::StartApp => {
            app_start(app, state, name).await?;
        }
    }
    Ok(fix.label)
}

/// Changes one of the .env settings the Overview shows (APP_URL…).
#[tauri::command]
pub(crate) async fn set_project_env(state: State<'_, AppState>, name: String, key: String, value: String) -> Result<()> {
    let (_, layout) = state.snapshot().await;
    manage::set_env_value(&project_dir(&layout, &name)?, &key, &value)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectFileInfo {
    pub(crate) file: projectfile::ProjectFile,
    /// The project's setup is already what the file says.
    pub(crate) matches: bool,
}

/// The project's lamprabit.toml, if it has one, and whether its setup already matches it.
#[tauri::command]
pub(crate) async fn project_file(state: State<'_, AppState>, name: String) -> Result<Option<ProjectFileInfo>> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let Some(file) = projectfile::read(&dir)? else { return Ok(None) };
    let parent = format!("{}.{}", sites::label(&name).unwrap_or_default(), settings.tld);
    let current = projectfile::ProjectFile::from_settings(&settings.sites.get(&name).cloned().unwrap_or_default(), &parent);
    Ok(Some(ProjectFileInfo { matches: current == file, file }))
}

/// Writes the project's setup to lamprabit.toml in its folder.
#[tauri::command]
pub(crate) async fn save_project_file(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<()> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let parent = format!("{}.{}", sites::label(&name).unwrap_or_default(), settings.tld);
    let file = projectfile::ProjectFile::from_settings(&settings.sites.get(&name).cloned().unwrap_or_default(), &parent);
    projectfile::write(&dir, &file)?;
    logs::system(&app, logs::Level::Info, format!("Saved {name}'s setup to {}", projectfile::FILE));
    Ok(())
}

/// Stops everything running for a project: its app, processes, command and tunnels.
pub(crate) async fn stop_project(app: &AppHandle, state: &AppState, name: &str) {
    let (settings, _) = state.snapshot().await;
    state.apps.stop_processes(app, &settings, name, false).await;
    state.apps.stop_command(name).await;
    state.apps.stop(app, name).await;
    for tunnel in state.tunnels.list().await.into_iter().filter(|t| t.site.as_deref() == Some(name)) {
        state.tunnels.stop(app, &tunnel.id).await;
    }
}

/// Renames a project's folder, moving its settings and subdomains along. Its database
/// keeps its name (MySQL can't rename one); the .env still points at it.
#[tauri::command]
pub(crate) async fn rename_project(app: AppHandle, state: State<'_, AppState>, name: String, new_name: String) -> Result<()> {
    let _op = state.ops.lock().await;
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let new_name = new_name.trim().to_string();
    if new_name == name {
        return Ok(());
    }
    // Only the case changes: Windows sees the same folder, so skip the "taken" check.
    if !new_name.eq_ignore_ascii_case(&name) {
        manage::check_project_name(&layout, &new_name)?;
    }
    stop_project(&app, &state, &name).await;
    let target = layout.www().join(&new_name);
    std::fs::rename(&dir, &target).map_err(|e| Error::msg(format!("Couldn't rename {name}: {e}. Close editors, terminals and anything else using its folder, then try again.")))?;
    let old_suffix = format!(".{}.{}", sites::label(&name).unwrap_or_default(), settings.tld);
    let new_suffix = format!(".{}.{}", sites::label(&new_name).unwrap_or_default(), settings.tld);
    state
        .update_settings(|s| {
            if let Some(mut own) = s.sites.remove(&name) {
                for d in &mut own.domains {
                    if let Some(sub) = d.host.strip_suffix(&old_suffix) {
                        d.host = format!("{sub}{new_suffix}");
                    }
                }
                s.sites.insert(new_name.clone(), own);
            }
            // Profiles name processes as "project/process".
            for profile in &mut s.profiles {
                for key in &mut profile.processes {
                    if let Some(process) = key.strip_prefix(&format!("{name}/")) {
                        *key = format!("{new_name}/{process}");
                    }
                }
            }
        })
        .await?;
    logs::system(&app, logs::Level::Info, format!("Renamed {name} to {new_name}"));
    sync_sites(&app, &state).await
}

/// Deletes a project: stops it, drops its database if asked, and moves its folder to the
/// Recycle Bin (Trash), so it can still be brought back.
#[tauri::command]
pub(crate) async fn delete_project(app: AppHandle, state: State<'_, AppState>, name: String, drop_database: bool) -> Result<()> {
    let _op = state.ops.lock().await;
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    stop_project(&app, &state, &name).await;
    if drop_database {
        let database = sites::scan(&settings, &layout).into_iter().find(|s| s.name == name).and_then(|s| s.database);
        if let (Some(database), Some(engine)) = (database, manage::engine(&settings, &dir)) {
            if let Some(kind) = engine.service()
                && !state.supervisor.is_running(kind).await {
                    return Err(Error::msg(format!("Start {} to drop {database}, or delete the project without its database.", engine.label())));
                }
            manage::drop(&settings, &layout, engine, &database).await?;
            logs::system(&app, logs::Level::Info, format!("Dropped database {database}"));
        }
    }
    let folder = dir.clone();
    tauri::async_runtime::spawn_blocking(move || trash::delete(&folder))
        .await
        .map_err(|e| Error::msg(e.to_string()))?
        .map_err(|e| Error::msg(format!("Couldn't move {name} to the Recycle Bin: {e}. Close editors, terminals and anything else using its folder, then try again.")))?;
    state.update_settings(|s| {
        s.sites.remove(&name);
        for profile in &mut s.profiles {
            profile.processes.retain(|k| !k.starts_with(&format!("{name}/")));
        }
    })
    .await?;
    logs::system(&app, logs::Level::Info, format!("Deleted {name} (moved to the Recycle Bin)"));
    sync_sites(&app, &state).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectFacts {
    #[serde(flatten)]
    pub(crate) facts: facts::Facts,
    /// The template it was created from (its name), when LampRabit created it.
    pub(crate) template: Option<String>,
}

/// Everything the Overview shows about a project beyond what the site list has.
#[tauri::command]
pub(crate) async fn project_facts(state: State<'_, AppState>, name: String) -> Result<ProjectFacts> {
    let (settings, layout) = state.snapshot().await;
    let dir = project_dir(&layout, &name)?;
    let site = sites::scan(&settings, &layout).into_iter().find(|s| s.name == name);
    let own = settings.sites.get(&name);
    let pm = own.and_then(|s| s.app.as_ref()).and_then(|a| a.package_manager).unwrap_or_else(|| apps::detect_package_manager(&dir));
    let template = own.and_then(|s| s.template.clone()).map(|id| {
        let id = id.split('@').next().unwrap_or(&id).to_string();
        templates::find(&layout, &id).map(|(t, _)| t.name).unwrap_or(id)
    });
    Ok(ProjectFacts { facts: facts::read(&dir, site.and_then(|s| s.framework), pm.id()).await, template })
}
