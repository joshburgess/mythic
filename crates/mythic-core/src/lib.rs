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
pub mod output_format;
pub mod page;
pub mod pagination;
pub mod plugin;
pub mod redirects;
pub mod related;
pub mod remote;
pub mod rhai_plugin;
pub mod search;
pub mod sitemap;
pub mod summary;
pub mod taxonomy;
