//! Chess-domain types and position representation.

mod bitboard;
mod fen;
mod moves;
mod position;
mod types;

pub use bitboard::Bitboard;
pub use fen::FenError;
pub use moves::{Move, MoveKind};
pub use position::Position;
pub use types::{CastleSide, CastlingRights, Color, Piece, PieceKind, Square, SquareParseError};
