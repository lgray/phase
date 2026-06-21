//! Offline game-state analysis used by the infinite-combo detector.
//!
//! This module is **purely additive** and changes no game behavior. It provides
//! the measurement substrate the net-progress loop detector is built on:
//!
//! - [`ResourceVector`] — a snapshot/delta of the *monotone* resources a loop
//!   can pump (mana, life, damage, library size, tokens, draws, triggers,
//!   counters, …). See [`resource`].
//! - [`loop_states_equal_modulo_resources`] — the **complement** of the existing
//!   strict CR 104.4b loop equality (`types::game_state::loop_states_equal`):
//!   board/zones/tap-state must be identical, but the monotone resources are
//!   allowed to differ. This is what distinguishes a *net-progress* (CR 732.2)
//!   loop from a *mandatory-draw* (CR 104.4b) loop.
//!
//! The strict comparison treats differing life/damage/counters as different
//! states (correct for a mandatory loop → draw). The detector needs the inverse:
//! "same board, resources may differ" → a beneficial loop that should be
//! shortcut (CR 732.2a) rather than drawn (CR 104.4b / CR 732.4).
//!
//! - [`sim`] — the offline simulation harness ([`LoopProbe`] / [`accumulate_events`])
//!   that drives `GameRunner::act` and *feeds* the event-fed `ResourceVector`
//!   axes (damage, tokens, draws, casts, triggers) from the runner's event
//!   stream, which a single `GameState` snapshot cannot supply.

pub mod resource;
pub mod sim;

pub use resource::{
    loop_states_equal_modulo_resources, CounterClass, ObjectClass, ResourceVector, TriggerKind,
};
pub use sim::{accumulate_events, LoopProbe};
