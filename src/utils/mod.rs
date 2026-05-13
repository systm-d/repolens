//! # Utility Modules
//!
//! This module provides shared utility functions used throughout RepoLens.
//!
//! ## Submodules
//!
//! - [`command`] - Shell command execution utilities
//! - [`language_detection`] - Programming language detection from file extensions
//! - [`permissions`] - File system permission checking
//! - [`prerequisites`] - System prerequisites validation
//! - [`timing`] - Audit timing and performance measurement
//!
//! ## Language Detection
//!
//! Detect programming languages in a repository:
//!
//! ```rust,no_run
//! use repolens::utils::detect_languages;
//! use repolens::scanner::Scanner;
//! use std::path::PathBuf;
//!
//! let scanner = Scanner::new(PathBuf::from("."));
//! let languages = detect_languages(&scanner);
//! for lang in &languages {
//!     println!("Found language: {:?}", lang);
//! }
//! ```
//!
//! ## Timing
//!
//! Measure audit performance:
//!
//! ```rust
//! use repolens::utils::{Timer, format_duration};
//!
//! let timer = Timer::start();
//! // ... do work ...
//! let duration = timer.elapsed();
//! println!("Completed in {}", format_duration(duration));
//! ```

pub mod command;
pub mod language_detection;
pub mod permissions;
pub mod prerequisites;
pub mod timing;

pub use language_detection::{detect_languages, get_gitignore_entries_with_descriptions};
pub use timing::{AuditTiming, CategoryTiming, Timer, format_duration};
