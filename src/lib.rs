//! Core library for the chess engine.
//!
//! The executable is intentionally thin. Chess representation, move generation,
//! evaluation, search, and protocol handling will live behind library APIs so
//! they can be tested and benchmarked without spawning a subprocess.

#![forbid(unsafe_code)]

/// Chess-domain types and position representation.
pub mod chess;
/// Position evaluation implementations and score types.
pub mod eval;
/// Universal Chess Interface protocol support.
pub mod uci;

/// Metadata identifying this engine build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineInfo {
    /// Human-readable engine name.
    pub name: &'static str,
    /// Package version supplied by Cargo.
    pub version: &'static str,
    /// Package author supplied by Cargo.
    pub author: &'static str,
}

impl EngineInfo {
    /// Returns metadata for the current engine build.
    #[must_use]
    pub const fn current() -> Self {
        Self {
            name: "Shrike Engine",
            version: env!("CARGO_PKG_VERSION"),
            author: env!("CARGO_PKG_AUTHORS"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EngineInfo;

    #[test]
    fn current_engine_info_matches_package_metadata() {
        let info = EngineInfo::current();

        assert_eq!(info.name, "Shrike Engine");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.author, "yaarek7");
    }
}
