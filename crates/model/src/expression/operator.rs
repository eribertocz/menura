//! Operators of the accepted subset (E0F-2.2).

/// A prefix operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
    /// Arithmetic `+`.
    Plus,
    /// Arithmetic `-`.
    Minus,
    /// Logical `NOT`.
    Not,
}

/// An infix operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    /// `+`.
    Add,
    /// `-`.
    Subtract,
    /// `*`.
    Multiply,
    /// `/`, true division also between integers (E0F-2.5).
    Divide,
    /// `%`.
    Modulo,
    /// `||`, text concatenation.
    Concatenate,
    /// `=`.
    Equal,
    /// `<>`.
    NotEqual,
    /// `<`.
    Less,
    /// `<=`.
    LessOrEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterOrEqual,
    /// `AND`.
    And,
    /// `OR`.
    Or,
}

/// Whether a pattern match is case sensitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LikeCase {
    /// `LIKE`.
    Sensitive,
    /// `ILIKE`.
    Insensitive,
}
