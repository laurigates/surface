//! surf-core: pure parse/resolve/hash logic for Surface. No I/O, no network.

pub mod anchor;
pub mod lang;
pub mod resolve;

pub use anchor::{parse_anchor, Anchor, AnchorParseError, Segment};
pub use lang::Lang;
pub use resolve::{resolve, ResolveError, Span};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
