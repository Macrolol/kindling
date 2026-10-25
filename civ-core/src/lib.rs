//! Kindling (working name) simulation core: shared by the native app, the headless runner and
//! the web build. Deterministic per seed; no threads or clocks inside the step.
pub mod analysis;
pub mod band;
pub mod knowledge;
pub mod map;
pub mod materials;
pub mod params;
pub mod physics;
pub mod rng;
pub mod save;
pub mod world;
#[cfg(test)]
mod tests;

pub use params::{parse_starts, Config, ParamChange, Params, StartCheck};
pub use save::ViewState;
pub use world::World;

#[doc(hidden)]
pub fn diag_flag(k: &str) -> bool {
    static F: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    F.get_or_init(|| std::env::var("KDIAG").unwrap_or_else(|_| "nokin".into())).split(',').any(|x| x == k)
}
