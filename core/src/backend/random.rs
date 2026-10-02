//! An embedder's random source for AVM1.

/// Replaces the player's own generator for AVM1's `random(n)` action and `Math.random()`, so
/// an embedder can reproduce another runtime's generator (and share one between players).
/// AVM2's `Math.random()` keeps the player's generator.
pub trait Avm1RandomBackend {
    /// The `random` action (0x30): `max` is the popped argument coerced to a number; the
    /// result is pushed as an integer.
    fn random_action(&mut self, max: f64) -> i32;

    /// `Math.random()`.
    fn math_random(&mut self) -> f64;
}
