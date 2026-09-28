//! Chess-domain types and position representation.

mod bitboard;
mod fen;
mod position;
mod types;

pub use bitboard::Bitboard;
pub use fen::FenError;
pub use position::Position;
pub use types::{CastleSide, CastlingRights, Color, Piece, PieceKind, Square, SquareParseError};
