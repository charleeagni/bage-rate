//! Desktop Target composition. Everything this crate composes — Migrations,
//! generated Models, Store access, and the App Schema — lives in `app-schema`,
//! which links no Tauri code.

pub mod composition;
pub mod lockin;
pub mod lockin_settings;
pub mod open_url;
pub mod tray;
