//! `bytemuck`: `Log<f32>` and `Log<f64>` are plain old data.
//!
//! `Log` is `#[repr(transparent)]` over its float, so every bit pattern is a valid value
//! *for memory safety*. Two consequences are worth knowing:
//!
//! - The all-zero bit pattern is ln(x) = 0, i.e. [`Log::ONE`], **not** `Log::ZERO`. A
//!   `bytemuck::zeroed()` buffer of `Log`s is a buffer of ones.
//! - Casting arbitrary bytes can produce a NaN stored log, which bypasses the crate's NaN
//!   check exactly like [`Log::from_ln_unchecked`].
#![allow(unsafe_code)]

use bytemuck::{Pod, Zeroable};

use crate::log::Log;

// SAFETY: `Log<F>` is `#[repr(transparent)]` over `F`, which is `Zeroable`. The zero
// pattern is a valid (if surprising: it is ONE) `Log`.
unsafe impl Zeroable for Log<f32> {}
// SAFETY: as above.
unsafe impl Zeroable for Log<f64> {}
// SAFETY: `#[repr(transparent)]` over a `Pod` float: no padding, every bit pattern is a
// valid (memory-safe) value, and the type is `Copy + 'static`.
unsafe impl Pod for Log<f32> {}
// SAFETY: as above.
unsafe impl Pod for Log<f64> {}
