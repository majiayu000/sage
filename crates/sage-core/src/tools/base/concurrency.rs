//! Concurrency control for tool execution

/// Concurrency mode for tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConcurrencyMode {
    /// Tool can run in parallel with any other tool
    #[default]
    Parallel,

    /// Tool must run sequentially (one at a time globally)
    Sequential,

    /// Tool can run in parallel but with a maximum count
    Limited(usize),

    /// Tool can run in parallel but not with tools of the same type
    ExclusiveByType,
}
