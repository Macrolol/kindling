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
#[cfg(test)]
mod tests;
pub mod world;

pub use params::{parse_starts, Config, ParamChange, Params, StartCheck};
pub use save::ViewState;
pub use world::World;
