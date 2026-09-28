//! Chess-domain types and position representation.

mod attacks;
mod bitboard;
mod fen;
pub(crate) mod movegen;
mod moves;
mod perft;
mod position;
mod types;

pub use bitboard::Bitboard;
pub use fen::FenError;
pub use movegen::{MoveError, PositionError};
pub use moves::{Move, MoveKind};
pub use perft::perft;
pub use position::{Position, Undo};
pub use types::{CastleSide, CastlingRights, Color, Piece, PieceKind, Square, SquareParseError};
