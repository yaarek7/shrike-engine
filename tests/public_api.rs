//! Integration tests for the engine's public API.

use chess_engine::EngineInfo;

#[test]
fn engine_metadata_is_available_to_external_consumers() {
    let info = EngineInfo::current();

    assert!(!info.name.is_empty());
    assert!(!info.version.is_empty());
}
