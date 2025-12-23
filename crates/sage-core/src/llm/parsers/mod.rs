//! Response parsers for different providers

pub mod responses;

#[cfg(test)]
mod responses_tests;

pub use responses::ResponseParser;
