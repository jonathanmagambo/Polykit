//! Global string interning for reducing memory allocations.
//!
//! Provides a thread-safe string interner that deduplicates strings
//! across the entire codebase, reducing memory usage and enabling
//! fast pointer-based string comparisons.

use dashmap::DashMap;
use std::sync::Arc;

/// Thread-safe string interner using DashMap.
pub struct StringInterner {
    strings: DashMap<String, Arc<str>>,
}

impl StringInterner {
    /// Creates a new string interner.
    pub fn new() -> Self {
        Self {
            strings: DashMap::new(),
        }
    }

    /// Interns a string slice, returning an Arc<str>.
    ///
    /// If the string has been interned before, returns the existing Arc.
    /// Otherwise, creates a new entry and returns it.
    pub fn intern(&self, s: &str) -> Arc<str> {
        self.strings
            .entry(s.to_string())
            .or_insert_with(|| Arc::from(s))
            .value()
            .clone()
    }

    /// Interns an owned String, returning an Arc<str>.
    ///
    /// More efficient than `intern` when you already have an owned String.
    pub fn intern_owned(&self, s: String) -> Arc<str> {
        self.strings
            .entry(s.clone())
            .or_insert_with(|| Arc::from(s))
            .value()
            .clone()
    }

    /// Returns the number of interned strings.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Returns true if no strings have been interned.
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Global thread-safe string interner.
///
/// This is a singleton instance shared across all threads.
/// All strings interned through this instance are deduplicated.
pub static GLOBAL_INTERNER: once_cell::sync::Lazy<StringInterner> =
    once_cell::sync::Lazy::new(StringInterner::new);

/// Interns a string slice using the global interner.
///
/// This is a convenience function that uses the global `GLOBAL_INTERNER`.
#[inline]
pub fn intern(s: &str) -> Arc<str> {
    GLOBAL_INTERNER.intern(s)
}

/// Interns an owned String using the global interner.
///
/// This is a convenience function that uses the global `GLOBAL_INTERNER`.
#[inline]
pub fn intern_owned(s: String) -> Arc<str> {
    GLOBAL_INTERNER.intern_owned(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_deduplication() {
        let interner = StringInterner::new();
        let s1 = interner.intern("hello");
        let s2 = interner.intern("hello");
        let s3 = interner.intern("world");

        assert!(Arc::ptr_eq(&s1, &s2));
        assert!(!Arc::ptr_eq(&s1, &s3));
        assert_eq!(interner.len(), 2);
    }

    #[test]
    fn test_global_intern() {
        let s1 = intern("test");
        let s2 = intern("test");
        let s3 = intern("other");

        assert!(Arc::ptr_eq(&s1, &s2));
        assert!(!Arc::ptr_eq(&s1, &s3));
    }

    #[test]
    fn test_intern_owned() {
        let interner = StringInterner::new();
        let s1 = interner.intern_owned("owned".to_string());
        let s2 = interner.intern("owned");

        assert!(Arc::ptr_eq(&s1, &s2));
    }
}
