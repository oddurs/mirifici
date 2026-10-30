//! Optional integrations with other crates, each behind a feature of the same name.

#[cfg(feature = "approx")]
mod approx;
#[cfg(feature = "bytemuck")]
mod bytemuck;
#[cfg(feature = "num-traits")]
mod num_traits;
#[cfg(feature = "serde")]
pub(crate) mod serde;
