//! SIMD-optimized utilities for performance-critical operations.
//!
//! This module provides SIMD-accelerated functions for common operations:
//! - String comparison
//! - ASCII validation
//! - Byte searching
//! - Byte counting
//!
//! SIMD implementations are architecture-specific and automatically selected at compile time.
//! Falls back to scalar implementations when SIMD is not available.

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(target_arch = "x86")]
use std::arch::x86::*;

/// Fast string comparison using SIMD when available.
///
/// Uses architecture-specific SIMD instructions on:
/// - ARM64/aarch64 (Apple Silicon, ARM servers)
/// - x86_64 (Intel/AMD with SSE2)
/// - x86 (32-bit Intel/AMD with SSE2)
///
/// Falls back to standard comparison on other architectures or short strings.
#[inline]
pub fn fast_str_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    if a.len() < 16 {
        return a == b;
    }

    #[cfg(target_arch = "aarch64")]
    {
        fast_str_eq_simd_aarch64(a.as_bytes(), b.as_bytes())
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        if is_x86_feature_detected!("sse2") {
            unsafe { fast_str_eq_simd_x86(a.as_bytes(), b.as_bytes()) }
        } else {
            a == b
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64", target_arch = "x86")))]
    {
        a == b
    }
}

#[cfg(target_arch = "aarch64")]
#[inline]
fn fast_str_eq_simd_aarch64(a: &[u8], b: &[u8]) -> bool {
    let len = a.len();
    let mut offset = 0;

    unsafe {
        while offset + 16 <= len {
            let a_chunk = vld1q_u8(a.as_ptr().add(offset));
            let b_chunk = vld1q_u8(b.as_ptr().add(offset));
            let cmp = vceqq_u8(a_chunk, b_chunk);
            let mask = vminvq_u8(cmp);
            
            if mask != 255 {
                return false;
            }
            
            offset += 16;
        }

        #[allow(clippy::needless_range_loop)]
        for i in offset..len {
            if a[i] != b[i] {
                return false;
            }
        }

        true
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "sse2")]
#[inline]
unsafe fn fast_str_eq_simd_x86(a: &[u8], b: &[u8]) -> bool {
    let len = a.len();
    let mut offset = 0;

    while offset + 16 <= len {
        let a_chunk = _mm_loadu_si128(a.as_ptr().add(offset) as *const __m128i);
        let b_chunk = _mm_loadu_si128(b.as_ptr().add(offset) as *const __m128i);
        let cmp = _mm_cmpeq_epi8(a_chunk, b_chunk);
        let mask = _mm_movemask_epi8(cmp);
        
        if mask != 0xFFFF {
            return false;
        }
        
        offset += 16;
    }

    #[allow(clippy::needless_range_loop)]
    for i in offset..len {
        if a[i] != b[i] {
            return false;
        }
    }

    true
}

/// Fast check if string contains only ASCII characters using SIMD.
#[inline]
pub fn is_ascii_fast(s: &[u8]) -> bool {
    if s.is_empty() {
        return true;
    }

    if s.len() < 16 {
        return s.iter().all(|&b| b < 128);
    }

    #[cfg(target_arch = "aarch64")]
    {
        is_ascii_simd_aarch64(s)
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        if is_x86_feature_detected!("sse2") {
            unsafe { is_ascii_simd_x86(s) }
        } else {
            s.iter().all(|&b| b < 128)
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64", target_arch = "x86")))]
    {
        s.iter().all(|&b| b < 128)
    }
}

#[cfg(target_arch = "aarch64")]
#[inline]
fn is_ascii_simd_aarch64(s: &[u8]) -> bool {
    let len = s.len();
    let mut offset = 0;

    unsafe {
        let ascii_mask = vdupq_n_u8(0x80);

        while offset + 16 <= len {
            let chunk = vld1q_u8(s.as_ptr().add(offset));
            let test = vtstq_u8(chunk, ascii_mask);
            let any_high = vmaxvq_u8(test);
            
            if any_high != 0 {
                return false;
            }
            
            offset += 16;
        }

        #[allow(clippy::needless_range_loop)]
        for i in offset..len {
            if s[i] >= 128 {
                return false;
            }
        }

        true
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "sse2")]
#[inline]
unsafe fn is_ascii_simd_x86(s: &[u8]) -> bool {
    let len = s.len();
    let mut offset = 0;

    let ascii_mask = _mm_set1_epi8(0x80u8 as i8);

    while offset + 16 <= len {
        let chunk = _mm_loadu_si128(s.as_ptr().add(offset) as *const __m128i);
        let test = _mm_and_si128(chunk, ascii_mask);
        let mask = _mm_movemask_epi8(test);
        
        if mask != 0 {
            return false;
        }
        
        offset += 16;
    }

    #[allow(clippy::needless_range_loop)]
    for i in offset..len {
        if s[i] >= 128 {
            return false;
        }
    }

    true
}

/// Fast byte search using SIMD.
#[inline]
pub fn find_byte_fast(haystack: &[u8], needle: u8) -> Option<usize> {
    if haystack.is_empty() {
        return None;
    }

    if haystack.len() < 16 {
        return haystack.iter().position(|&b| b == needle);
    }

    #[cfg(target_arch = "aarch64")]
    {
        find_byte_simd_aarch64(haystack, needle)
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        if is_x86_feature_detected!("sse2") {
            unsafe { find_byte_simd_x86(haystack, needle) }
        } else {
            haystack.iter().position(|&b| b == needle)
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64", target_arch = "x86")))]
    {
        haystack.iter().position(|&b| b == needle)
    }
}

#[cfg(target_arch = "aarch64")]
#[inline]
fn find_byte_simd_aarch64(haystack: &[u8], needle: u8) -> Option<usize> {
    let len = haystack.len();
    let mut offset = 0;

    unsafe {
        let needle_vec = vdupq_n_u8(needle);

        while offset + 16 <= len {
            let chunk = vld1q_u8(haystack.as_ptr().add(offset));
            let cmp = vceqq_u8(chunk, needle_vec);
            let mask = vmaxvq_u8(cmp);
            
            if mask != 0 {
                #[allow(clippy::needless_range_loop)]
                for i in 0..16 {
                    if haystack[offset + i] == needle {
                        return Some(offset + i);
                    }
                }
            }
            
            offset += 16;
        }

        #[allow(clippy::needless_range_loop)]
        for i in offset..len {
            if haystack[i] == needle {
                return Some(i);
            }
        }

        None
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "sse2")]
#[inline]
unsafe fn find_byte_simd_x86(haystack: &[u8], needle: u8) -> Option<usize> {
    let len = haystack.len();
    let mut offset = 0;

    let needle_vec = _mm_set1_epi8(needle as i8);

    while offset + 16 <= len {
        let chunk = _mm_loadu_si128(haystack.as_ptr().add(offset) as *const __m128i);
        let cmp = _mm_cmpeq_epi8(chunk, needle_vec);
        let mask = _mm_movemask_epi8(cmp);
        
        if mask != 0 {
            #[allow(clippy::needless_range_loop)]
            for i in 0..16 {
                if haystack[offset + i] == needle {
                    return Some(offset + i);
                }
            }
        }
        
        offset += 16;
    }

    #[allow(clippy::needless_range_loop)]
    for i in offset..len {
        if haystack[i] == needle {
            return Some(i);
        }
    }

    None
}

/// Fast count of specific byte in slice using SIMD.
#[inline]
pub fn count_byte_fast(haystack: &[u8], needle: u8) -> usize {
    if haystack.is_empty() {
        return 0;
    }

    if haystack.len() < 16 {
        return haystack.iter().filter(|&&b| b == needle).count();
    }

    #[cfg(target_arch = "aarch64")]
    {
        count_byte_simd_aarch64(haystack, needle)
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        if is_x86_feature_detected!("sse2") {
            unsafe { count_byte_simd_x86(haystack, needle) }
        } else {
            haystack.iter().filter(|&&b| b == needle).count()
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64", target_arch = "x86")))]
    {
        haystack.iter().filter(|&&b| b == needle).count()
    }
}

#[cfg(target_arch = "aarch64")]
#[inline]
fn count_byte_simd_aarch64(haystack: &[u8], needle: u8) -> usize {
    let len = haystack.len();
    let mut offset = 0;
    let mut count = 0;

    unsafe {
        let needle_vec = vdupq_n_u8(needle);
        let ones = vdupq_n_u8(1);

        while offset + 16 <= len {
            let chunk = vld1q_u8(haystack.as_ptr().add(offset));
            let cmp = vceqq_u8(chunk, needle_vec);
            let masked = vandq_u8(cmp, ones);
            count += vaddvq_u8(masked) as usize;
            offset += 16;
        }

        #[allow(clippy::needless_range_loop)]
        for i in offset..len {
            if haystack[i] == needle {
                count += 1;
            }
        }

        count
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "sse2")]
#[inline]
unsafe fn count_byte_simd_x86(haystack: &[u8], needle: u8) -> usize {
    let len = haystack.len();
    let mut offset = 0;
    let mut count = 0;

    let needle_vec = _mm_set1_epi8(needle as i8);

    while offset + 16 <= len {
        let chunk = _mm_loadu_si128(haystack.as_ptr().add(offset) as *const __m128i);
        let cmp = _mm_cmpeq_epi8(chunk, needle_vec);
        let mask = _mm_movemask_epi8(cmp);
        count += mask.count_ones() as usize;
        offset += 16;
    }

    #[allow(clippy::needless_range_loop)]
    for i in offset..len {
        if haystack[i] == needle {
            count += 1;
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_str_eq() {
        assert!(fast_str_eq("hello", "hello"));
        assert!(!fast_str_eq("hello", "world"));
        assert!(!fast_str_eq("hello", "hello!"));
        
        let long_str = "a".repeat(100);
        assert!(fast_str_eq(&long_str, &long_str));
        assert!(!fast_str_eq(&long_str, &"b".repeat(100)));
    }

    #[test]
    fn test_is_ascii_fast() {
        assert!(is_ascii_fast(b"hello world"));
        assert!(is_ascii_fast(b"0123456789abcdefghijklmnop"));
        assert!(!is_ascii_fast("hello 世界".as_bytes()));
    }

    #[test]
    fn test_find_byte_fast() {
        assert_eq!(find_byte_fast(b"hello", b'e'), Some(1));
        assert_eq!(find_byte_fast(b"hello world!", b'w'), Some(6));
        assert_eq!(find_byte_fast(b"hello", b'x'), None);
        
        let mut long_bytes = b"a".repeat(100);
        long_bytes.push(b'b');
        assert_eq!(find_byte_fast(&long_bytes, b'b'), Some(100));
    }

    #[test]
    fn test_count_byte_fast() {
        assert_eq!(count_byte_fast(b"hello", b'l'), 2);
        assert_eq!(count_byte_fast(b"aaabbbccc", b'b'), 3);
        assert_eq!(count_byte_fast(b"hello", b'x'), 0);
        
        let long_bytes = vec![b'a'; 100];
        assert_eq!(count_byte_fast(&long_bytes, b'a'), 100);
    }
}
