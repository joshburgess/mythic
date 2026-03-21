//! Core library for the Mythic static site generator.
//!
//! Provides site configuration, content discovery, the build pipeline,
//! and orchestration across the other crates.

pub mod config;
pub mod content;
pub mod build;
pub mod page;
