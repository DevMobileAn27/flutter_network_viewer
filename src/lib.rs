//! Capture engine shared by the future desktop UI, VM transport and MCP server.
pub mod capture;

pub mod json;

pub mod json_find;

pub mod console;

pub mod form_body;

pub mod image_body;

pub mod body_color;

pub mod settings;

pub mod vm;

pub mod lifecycle;

pub mod vm_worker;

pub mod coordinator;

pub mod discovery;

pub mod desktop;

pub mod desktop_host;

pub mod desktop_json;

pub mod desktop_body;

#[cfg(target_os = "macos")]
mod image_thumbnail_macos;

pub mod desktop_support;

pub mod mcp;

pub mod diagnostics;
pub mod resources;

pub mod updater;

pub mod desktop_style;

pub mod vintage;

pub mod app_colors;
