//! Kindling (working name) simulation core: shared by the native app, the headless runner and
//! the web build. Deterministic per seed; no threads or clocks inside the step.
pub mod analysis;
pub mod band;
pub mod effects;
pub mod archive;
pub mod codec;
pub mod data;
pub mod knowledge;
pub mod map;
pub mod materials;
pub mod params;
pub mod physics;
pub mod rng;
pub mod save;
pub mod world;
#[cfg(test)]
mod probe_diffusion;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_diffusion;
#[cfg(test)]
mod tests_skills;

pub use params::{parse_starts, Config, ParamChange, Params, StartCheck};
pub use save::ViewState;
pub use world::World;
