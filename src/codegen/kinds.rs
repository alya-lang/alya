//! Canonical runtime value-kind tags (alya-lang/alya#39).
//!
//! Single source of truth for the numeric kind tags carried in map
//! entries (packed), index-read scratch registers (x64 `%edx`, ARM64
//! `w1`), and the runtime classifier result. Array slots reuse these
//! in Phase 1. Values are ABI: entries live in memory only, but every
//! emitter and consumer in this crate must agree on them.
use crate::ast::Expr;

pub const KIND_UNKNOWN: i64 = 0;
pub const KIND_INT: i64 = 1;
pub const KIND_FLOAT: i64 = 2;
pub const KIND_STRING: i64 = 3;
pub const KIND_ARRAY: i64 = 4;
pub const KIND_MAP: i64 = 5;
pub const KIND_STRUCT: i64 = 6;

/// Kind of a literal value for tag recording (map `set`, and Phase 1
/// array `push`). Returns `None` for dynamic values, which keep the
/// unknown tag (0) so readers fall back to inference.
pub fn kind_of_literal(value: &Expr) -> Option<i64> {
    match value {
        Expr::String(_) => Some(KIND_STRING),
        Expr::Float(_) => Some(KIND_FLOAT),
        Expr::Number(_) => Some(KIND_INT),
        Expr::Array(_) => Some(KIND_ARRAY),
        Expr::Map(_) => Some(KIND_MAP),
        Expr::StructInit { .. } => Some(KIND_STRUCT),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_registry_values_are_stable() {
        // ABI: numeric values flow into emitted machine code.
        assert_eq!(KIND_UNKNOWN, 0);
        assert_eq!(KIND_INT, 1);
        assert_eq!(KIND_FLOAT, 2);
        assert_eq!(KIND_STRING, 3);
        assert_eq!(KIND_ARRAY, 4);
        assert_eq!(KIND_MAP, 5);
        assert_eq!(KIND_STRUCT, 6);
    }
}
