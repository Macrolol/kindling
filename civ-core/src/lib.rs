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

pub use params::{Config, ParamChange, Params};
pub use save::ViewState;
pub use world::World;
