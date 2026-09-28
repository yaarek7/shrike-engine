//! Core library for the chess engine.
//!
//! The executable is intentionally thin. Chess representation, move generation,
//! evaluation, search, and protocol handling will live behind library APIs so
//! they can be tested and benchmarked without spawning a subprocess.

#![forbid(unsafe_code)]

/// Chess-domain types and position representation.
pub mod chess;

/// Metadata identifying this engine build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineInfo {
    /// Human-readable engine name.
    pub name: &'static str,
    /// Package version supplied by Cargo.
    pub version: &'static str,
}

impl EngineInfo {
    /// Returns metadata for the current engine build.
    #[must_use]
    pub const fn current() -> Self {
        Self {
            name: "Chess Engine",
            version: env!("CARGO_PKG_VERSION"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EngineInfo;

    #[test]
    fn current_engine_info_matches_package_metadata() {
        let info = EngineInfo::current();

        assert_eq!(info.name, "Chess Engine");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    }
}
