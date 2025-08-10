//! Thompson NFA Compiler with Two-Character Transitions
//! 
//! This library implements a Thompson NFA compiler that uses two-character transitions
//! instead of the traditional single-character transitions. Each transition matches based
//! on both the current character and the next character (single-character lookahead).
//! 
//! This design enables:
//! - Single-character lookahead for all transitions
//! - Efficient possessive repetition for single atom repetitions
//! - Better optimization opportunities for specific patterns


pub mod nfa;
pub mod compiler;
pub mod matcher;

pub use nfa::{NFA, State, StateId, TwoCharTransition, Fragment};
pub use compiler::Compiler;
pub use matcher::Matcher;

/// The result of compiling a regex to a two-character Thompson NFA
pub type CompileResult<T> = Result<T, CompileError>;

/// Errors that can occur during compilation
#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    /// The regex pattern is too complex to compile
    TooComplex,
    /// Unsupported regex feature
    UnsupportedFeature(String),
    /// Internal compilation error
    Internal(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::TooComplex => write!(f, "regex pattern is too complex"),
            CompileError::UnsupportedFeature(feature) => write!(f, "unsupported feature: {}", feature),
            CompileError::Internal(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl std::error::Error for CompileError {}