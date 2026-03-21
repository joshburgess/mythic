//! Core library for the Mythic static site generator.
//!
//! Provides site configuration, content discovery, the build pipeline,
//! and orchestration across the other crates.

pub mod build;
pub mod cache;
pub mod cascade;
pub mod config;
pub mod content;
pub mod data;
pub mod feed;
pub mod page;
pub mod taxonomy;
