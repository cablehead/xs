//! This module contains six implementations of WebSocket frame masking and
//! unmasking, all of them using the same algorithm and methods:
//!   - One AVX512-based implementation that masks 64 bytes per cycle (requires
//!     nightly rust)
//!   - One AVX2-based implementation that masks 32 bytes per cycle
//!   - One SSE2-based implementation that masks 16 bytes per cycle
//!   - One NEON-based implementation that masks 16 bytes per cycle (requires
//!     nightly rust on 32-bit ARM)
//!   - One AltiVec-based implementation that masks 16 bytes per cycle (requires
//!     nightly rust)
//!   - One IBM z13 vector facility based implementation that masks 16 bytes per
//!     cycle (requires nightly rust)
//!   - One LASX based implementation that masks 32 bytes per cycle (requires
//!     nightly rust)
//!   - A fallback implementation that masks 8 bytes per cycle
//!
//! The SIMD implementations will be used if CPU support for them is detected at
//! runtime or enabled at compile time via compiler flags.

/// (Un-)masks input bytes with the framing key using AVX512.
///
/// This will use a fallback implementation for less than 64 bytes. For
/// sufficiently large inputs, it masks in chunks of 64 bytes per
/// instruction, applying the fallback method on all remaining data.
#[cfg(all(feature = "nightly", any(target_arch = "x86", target_arch = "x86_64")))]
#[allow(clippy::incompatible_msrv)] // nightly feature gated, stable since 1.89.0
#[target_feature(enable = "avx512f")]
unsafe fn frame_avx512(key: &mut [u8; 4], input: &mut [u8]) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{__m512i, _mm512_set1_epi32, _mm512_xor_si512};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{__m512i, _mm512_set1_epi32, _mm512_xor_si512};

    unsafe {
        let (prefix, aligned_data, suffix) = input.align_to_mut::<__m512i>();

        // Run fallback implementation on unaligned prefix data
        if !prefix.is_empty() {
            fallback_frame(key, prefix);
        }

        if !aligned_data.is_empty() {
            let mask = _mm512_set1_epi32(i32::from_ne_bytes(*key));

            for block in aligned_data {
                *block = _mm512_xor_si512(*block, mask);
            }
        }

        // Run fallback implementation on unaligned suffix data
        if !suffix.is_empty() {
            fallback_frame(key, suffix);
        }
    }
}

/// (Un-)masks input bytes with the framing key using AVX2.
///
/// This will use a fallback implementation for less than 32 bytes. For
/// sufficiently large inputs, it masks in chunks of 32 bytes per
/// instruction, applying the fallback method on all remaining data.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn frame_avx2(key: &mut [u8; 4], input: &mut [u8]) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{__m256i, _mm256_set1_epi32, _mm256_xor_si256};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{__m256i, _mm256_set1_epi32, _mm256_xor_si256};

    unsafe {
        let (prefix, aligned_data, suffix) = input.align_to_mut::<__m256i>();

        // Run fallback implementation on unaligned prefix data
        if !prefix.is_empty() {
            fallback_frame(key, prefix);
        }

        if !aligned_data.is_empty() {
            let mask = _mm256_set1_epi32(i32::from_ne_bytes(*key));

            for block in aligned_data {
                *block = _mm256_xor_si256(*block, mask);
            }
        }

        // Run fallback implementation on unaligned suffix data
        if !suffix.is_empty() {
            fallback_frame(key, suffix);
        }
    }
}

/// (Un-)masks input bytes with the framing key using SSE2.
///
/// This will use a fallback implementation for less than 16 bytes. For
/// sufficiently large inputs, it masks in chunks of 16 bytes per
/// instruction, applying the fallback method on all remaining data.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn frame_sse2(key: &mut [u8; 4], input: &mut [u8]) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{__m128i, _mm_set1_epi32, _mm_xor_si128};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{__m128i, _mm_set1_epi32, _mm_xor_si128};

    unsafe {
        let (prefix, aligned_data, suffix) = input.align_to_mut::<__m128i>();

        // Run fallback implementation on unaligned prefix data
        if !prefix.is_empty() {
            fallback_frame(key, prefix);
        }

        if !aligned_data.is_empty() {
            let mask = _mm_set1_epi32(i32::from_ne_bytes(*key));

            for block in aligned_data {
                *block = _mm_xor_si128(*block, mask);
            }
        }

        // Run fallback implementation on unaligned suffix data
        if !suffix.is_empty() {
            fallback_frame(key, suffix);
        }
    }
}

/// (Un-)masks input bytes with the framing key using NEON.
///
/// This will use a fallback implementation for less than 16 bytes. For
/// sufficiently large inputs, it masks in chunks of 16 bytes per
/// instruction, applying the fallback method on all remaining data.
#[cfg(any(all(feature = "nightly", target_arch = "arm"), target_arch = "aarch64"))]
#[target_feature(enable = "neon")]
unsafe fn frame_neon(key: &mut [u8; 4], input: &mut [u8]) {
    #[cfg(target_arch = "aarch64")]
    use std::arch::aarch64::{uint8x16_t, veorq_u8, vld1q_dup_s32, vreinterpretq_u8_s32};
    #[cfg(target_arch = "arm")]
    use std::arch::arm::{uint8x16_t, veorq_u8, vld1q_dup_s32, vreinterpretq_u8_s32};

    unsafe {
        let (prefix, aligned_data, suffix) = input.align_to_mut::<uint8x16_t>();

        // Run fallback implementation on unaligned prefix data
        if !prefix.is_empty() {
            fallback_frame(key, prefix);
        }

        if !aligned_data.is_empty() {
            let mask = vreinterpretq_u8_s32(vld1q_dup_s32(key.as_ptr() as *const i32));

            for block in aligned_data {
                *block = veorq_u8(*block, mask);
            }
        }

        // Run fallback implementation on unaligned suffix data
        if !suffix.is_empty() {
            fallback_frame(key, suffix);
        }
    }
}

/// (Un-)masks input bytes with the framing key using AltiVec.
///
/// This will use a fallback implementation for less than 16 bytes. For
/// sufficiently large inputs, it masks in chunks of 16 bytes per
/// instruction, applying the fallback method on all remaining data.
#[cfg(all(
    feature = "nightly",
    any(target_arch = "powerpc", target_arch = "powerpc64")
))]
#[target_feature(enable = "altivec")]
unsafe fn frame_altivec(key: &mut [u8; 4], input: &mut [u8]) {
    #[cfg(target_arch = "powerpc")]
    use std::arch::powerpc::{vec_splats, vec_xor, vector_unsigned_char};
    #[cfg(target_arch = "powerpc64")]
    use std::arch::powerpc64::{vec_splats, vec_xor, vector_unsigned_char};
    use std::mem::transmute;

    unsafe {
        let (prefix, aligned_data, suffix) = input.align_to_mut::<vector_unsigned_char>();

        // Run fallback implementation on unaligned prefix data
        if !prefix.is_empty() {
            fallback_frame(key, prefix);
        }

        if !aligned_data.is_empty() {
            // SAFETY: 4x i32 to 16x u8 is safe
            let mask: vector_unsigned_char = transmute(vec_splats(i32::from_ne_bytes(*key)));

            for block in aligned_data {
                *block = vec_xor(*block, mask);
            }
        }

        // Run fallback implementation on unaligned suffix data
        if !suffix.is_empty() {
            fallback_frame(key, suffix);
        }
    }
}

/// (Un-)masks input bytes with the framing key using s390x vectors.
///
/// This will use a fallback implementation for less than 16 bytes. For
/// sufficiently large inputs, it masks in chunks of 16 bytes per
/// instruction, applying the fallback method on all remaining data.
#[cfg(all(feature = "nightly", target_arch = "s390x"))]
#[target_feature(enable = "vector")]
unsafe fn frame_s390x_vector(key: &mut [u8; 4], input: &mut [u8]) {
    use std::{
        arch::s390x::{vec_splats, vec_xor, vector_signed_int, vector_unsigned_char},
        mem::transmute,
    };

    unsafe {
        let (prefix, aligned_data, suffix) = input.align_to_mut::<vector_unsigned_char>();

        // Run fallback implementation on unaligned prefix data
        if !prefix.is_empty() {
            fallback_frame(key, prefix);
        }

        if !aligned_data.is_empty() {
            // SAFETY: 4x i32 to 16x u8 is safe
            let mask: vector_unsigned_char = transmute(vec_splats::<i32, vector_signed_int>(
                i32::from_ne_bytes(*key),
            ));

            for block in aligned_data {
                *block = vec_xor(*block, mask);
            }
        }

        // Run fallback implementation on unaligned suffix data
        if !suffix.is_empty() {
            fallback_frame(key, suffix);
        }
    }
}

/// (Un-)masks input bytes with the framing key using LASX.
///
/// This will use a fallback implementation for less than 32 bytes. For
/// sufficiently large inputs, it masks in chunks of 32 bytes per
/// instruction, applying the fallback method on all remaining data.
#[cfg(all(feature = "nightly", target_arch = "loongarch64"))]
#[target_feature(enable = "lasx")]
unsafe fn frame_lasx_vector(key: &mut [u8; 4], input: &mut [u8]) {
    use std::{
        arch::loongarch64::{lasx_xvld, lasx_xvxor_v, v32u8},
        mem::transmute,
    };

    unsafe {
        let (prefix, aligned_data, suffix) = input.align_to_mut::<v32u8>();

        // Run fallback implementation on unaligned prefix data
        if !prefix.is_empty() {
            fallback_frame(key, prefix);
        }

        if !aligned_data.is_empty() {
            let key_vector = [i32::from_ne_bytes(*key); 8];
            // SAFETY: 32x i8 to 32x u8 is safe
            let mask: v32u8 = transmute(lasx_xvld::<0>(key_vector.as_ptr().cast()));

            for block in aligned_data {
                *block = lasx_xvxor_v(*block, mask);
            }
        }

        // Run fallback implementation on unaligned suffix data
        if !suffix.is_empty() {
            fallback_frame(key, suffix);
        }
    }
}

/// Rotates the mask in-place by a certain amount of bytes.
#[allow(clippy::cast_possible_truncation)] // offset % 4 is within u32 bounds
fn rotate_mask(key: &mut [u8; 4], offset: usize) {
    *key = u32::from_be_bytes(*key)
        .rotate_left((offset % key.len()) as u32 * u8::BITS)
        .to_be_bytes();
}

/// (Un-)masks input bytes with the framing key, one byte at once.
///
/// See [`fallback_frame`] for more details.
fn one_byte_at_once(key: &mut [u8; 4], input: &mut [u8]) {
    for (index, byte) in input.iter_mut().enumerate() {
        *byte ^= key[index % key.len()];
    }

    rotate_mask(key, input.len());
}

/// (Un-)masks input bytes with the framing key.
///
/// This is used as the internal implementation in non-SIMD builds and as a
/// fallback in SIMD builds.
fn fallback_frame(key: &mut [u8; 4], input: &mut [u8]) {
    let (prefix, aligned_data, suffix) = unsafe { input.align_to_mut::<u64>() };

    // Run fallback implementation on unaligned prefix data
    if !prefix.is_empty() {
        one_byte_at_once(key, prefix);
    }

    if !aligned_data.is_empty() {
        let masking_key = u64::from(u32::from_ne_bytes(*key));
        let mask = (masking_key << u32::BITS) | masking_key;

        for block in aligned_data {
            *block ^= mask;
        }
    }

    // Run fallback implementation on unaligned suffix data
    if !suffix.is_empty() {
        one_byte_at_once(key, suffix);
    }
}

/// (Un-)masks input bytes with the framing key.
#[inline]
pub fn frame(key: &mut [u8; 4], input: &mut [u8]) {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        use std::arch::is_x86_feature_detected;

        #[cfg(feature = "nightly")]
        if is_x86_feature_detected!("avx512f") {
            return unsafe { frame_avx512(key, input) };
        }

        if is_x86_feature_detected!("avx2") {
            return unsafe { frame_avx2(key, input) };
        } else if is_x86_feature_detected!("sse2") {
            return unsafe { frame_sse2(key, input) };
        }
    }

    #[cfg(all(feature = "nightly", target_arch = "arm"))]
    {
        use std::arch::is_arm_feature_detected;

        if is_arm_feature_detected!("neon") {
            return unsafe { frame_neon(key, input) };
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        use std::arch::is_aarch64_feature_detected;

        if is_aarch64_feature_detected!("neon") {
            return unsafe { frame_neon(key, input) };
        }
    }

    #[cfg(all(feature = "nightly", target_arch = "powerpc"))]
    {
        use std::arch::is_powerpc_feature_detected;

        if is_powerpc_feature_detected!("altivec") {
            return unsafe { frame_altivec(key, input) };
        }
    }

    #[cfg(all(feature = "nightly", target_arch = "powerpc64"))]
    {
        use std::arch::is_powerpc64_feature_detected;

        if is_powerpc64_feature_detected!("altivec") {
            return unsafe { frame_altivec(key, input) };
        }
    }

    #[cfg(all(feature = "nightly", target_arch = "s390x"))]
    {
        use std::arch::is_s390x_feature_detected;

        if is_s390x_feature_detected!("vector") {
            return unsafe { frame_s390x_vector(key, input) };
        }
    }

    #[cfg(all(feature = "nightly", target_arch = "loongarch64"))]
    {
        use std::arch::is_loongarch_feature_detected;

        if is_loongarch_feature_detected!("lasx") {
            return unsafe { frame_lasx_vector(key, input) };
        }
    }

    fallback_frame(key, input);
}

#[cfg(all(test, feature = "client", feature = "fastrand"))]
#[test]
fn test_mask() {
    use crate::rand::get_mask;

    let mut random_data: Vec<u8> = std::iter::repeat_with(|| fastrand::u8(..))
        .take(1024)
        .collect();
    // Mess around with the data to ensure we have unaligned input
    let data = &mut random_data[2..998];
    let mut data_clone = data.to_vec();
    let mut mask = [0; 4];
    get_mask(&mut mask);
    let mut mask2 = mask;
    frame(&mut mask, data);
    one_byte_at_once(&mut mask2, &mut data_clone);

    assert_eq!(&data, &data_clone);
}
