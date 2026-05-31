//! Web UI module: Loco-style Tera + HTMX + Alpine + Lily Design System.
//!
//! This module provides a server-rendered web UI on top of the existing
//! Axum-based REST API. It wires up:
//!
//! - [Loco.rs](https://loco.rs) — full-stack web framework conventions
//!   (config files in `config/`, app hooks in [`app`]).
//! - [Tera](https://keats.github.io/tera/) — templates in `assets/views/`.
//! - [HTMX](https://htmx.org) — server-driven interactivity from
//!   `assets/static/js/htmx.min.js`.
//! - [Alpine](https://alpinejs.dev) — light client-side reactivity from
//!   `assets/static/js/alpine.min.js`.
//! - [Lily Design System](https://lilydesignsystem.github.io) — styling
//!   from `assets/static/css/lily.css`.

pub mod app;
pub mod controllers;
pub mod routes;
pub mod views;

pub use routes::router;
pub use views::{engine, WebState};
