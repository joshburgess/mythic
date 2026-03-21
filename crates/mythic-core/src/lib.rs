//! Core library for the Mythic static site generator.
//!
//! Provides site configuration, content discovery, the build pipeline,
//! and orchestration across the other crates.

pub mod bench_utils;
pub mod build;
pub mod cache;
pub mod cascade;
pub mod check;
pub mod config;
pub mod content;
pub mod data;
pub mod feed;
pub mod i18n;
pub mod migrate;
pub mod page;
pub mod plugin;
pub mod rhai_plugin;
pub mod sitemap;
pub mod taxonomy;
