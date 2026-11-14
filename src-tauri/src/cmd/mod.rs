//! The Tauri commands (what the window, the command line and MCP call), one file per area.
//! lib.rs keeps the app's state, its start-up and the background loops.

mod settings;
mod services;
mod databases;
mod environment;
mod sites;
mod projects;
mod apps;
mod git;
mod control;
mod runtimes;
mod sharing;
mod logs;

pub(crate) use settings::*;
pub(crate) use services::*;
pub(crate) use databases::*;
pub(crate) use environment::*;
pub(crate) use sites::*;
pub(crate) use projects::*;
pub(crate) use apps::*;
pub(crate) use git::*;
pub(crate) use control::*;
pub(crate) use runtimes::*;
pub(crate) use sharing::*;
pub(crate) use logs::*;
