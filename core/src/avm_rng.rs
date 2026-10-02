use crate::backend::locale::LocaleBackend;
use crate::backend::random::Avm1RandomBackend;

// https://github.com/adobe/avmplus/blob/858d034a3bd3a54d9b70909386435cf4aec81d21/core/MathUtils.cpp#L1546
const C1: i32 = 1376312589;
const C2: i32 = 789221;
const C3: i32 = 15731;
const K_RANDOM_PURE_MAX: i32 = 0x7FFFFFFF;

const U_XOR_MASK: u32 = 0x48000000;

// This struct should not be cloned or copied.
#[derive(Default)]
pub struct AvmRng {
    u_value: u32,
    /// The embedder's source for AVM1's draws, when it supplied one.
    avm1: Option<Box<dyn Avm1RandomBackend>>,
}

impl std::fmt::Debug for AvmRng {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AvmRng")
            .field("u_value", &self.u_value)
            .field("avm1", &self.avm1.is_some())
            .finish()
    }
}

impl AvmRng {
    pub fn with_avm1_backend(avm1: Option<Box<dyn Avm1RandomBackend>>) -> Self {
        Self { u_value: 0, avm1 }
    }

    /// AVM1's `random` action: the embedder's answer, else a number in `[0, max)` (0 when
    /// `max` is not positive). `max` is clamped to the range [0, 2^31 - 1).
    pub fn avm1_random_action(&mut self, max: f64, locale: &dyn LocaleBackend) -> i32 {
        if let Some(b) = &mut self.avm1 {
            return b.random_action(max);
        }
        let max = max as i32;
        if max > 0 {
            self.generate_random_number(locale) % max
        } else {
            0
        }
    }

    /// AVM1's `Math.random()`: the embedder's answer, else avmplus' restricted set of values.
    pub fn avm1_math_random(&mut self, locale: &dyn LocaleBackend) -> f64 {
        if let Some(b) = &mut self.avm1 {
            return b.math_random();
        }
        // See https://github.com/adobe/avmplus/blob/858d034a3bd3a54d9b70909386435cf4aec81d21/core/MathUtils.cpp#L1731C24-L1731C44
        // This generated a restricted set of 'f64' values, which some SWFs implicitly rely on.
        const MAX_VAL: u32 = 0x7FFFFFFF;
        let rand = self.generate_random_number(locale);
        (rand as f64) / (MAX_VAL as f64 + 1f64)
    }

    fn init_with_seed(&mut self, seed: u32) {
        self.u_value = seed;
    }

    fn random_fast_next(&mut self) -> i32 {
        if (self.u_value & 1) != 0 {
            self.u_value = (self.u_value >> 1) ^ U_XOR_MASK;
        } else {
            self.u_value >>= 1;
        }
        self.u_value as i32
    }

    fn random_pure_hasher(&self, mut i_seed: i32) -> i32 {
        i_seed = ((i_seed << 13) ^ i_seed).wrapping_sub(i_seed >> 21);

        let mut i_result = i_seed.wrapping_mul(i_seed);
        i_result = i_result.wrapping_mul(C3);
        i_result = i_result.wrapping_add(C2);
        i_result = i_result.wrapping_mul(i_seed);
        i_result = i_result.wrapping_add(C1);
        i_result &= K_RANDOM_PURE_MAX;

        i_result = i_result.wrapping_add(i_seed);

        i_result = ((i_result << 13) ^ i_result).wrapping_sub(i_result >> 21);

        i_result
    }

    pub fn generate_random_number(&mut self, locale: &dyn LocaleBackend) -> i32 {
        // In avmplus, RNG is initialized on first use.
        if self.u_value == 0 {
            let seed = get_seed(locale);
            self.init_with_seed(seed);
        }

        let mut a_num = self.random_fast_next();

        a_num = self.random_pure_hasher(a_num.wrapping_mul(71));

        a_num & K_RANDOM_PURE_MAX
    }
}

// https://github.com/adobe-flash/avmplus/blob/65a05927767f3735db37823eebf7d743531f5d37/VMPI/PosixSpecificUtils.cpp#L18
fn get_seed(locale: &dyn LocaleBackend) -> u32 {
    locale.get_current_date_time().timestamp_micros() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::locale::DeterministicLocaleBackend;

    struct Counting(u32);

    impl Avm1RandomBackend for Counting {
        fn random_action(&mut self, max: f64) -> i32 {
            self.0 += 1;
            max as i32 - 1
        }

        fn math_random(&mut self) -> f64 {
            self.0 += 1;
            0.25
        }
    }

    #[test]
    fn avm1_draws_ask_the_backend() {
        let locale = DeterministicLocaleBackend::default();
        let mut rng = AvmRng::with_avm1_backend(Some(Box::new(Counting(0))));
        assert_eq!(rng.avm1_random_action(4.0, &locale), 3);
        assert_eq!(rng.avm1_math_random(&locale), 0.25);
        // Without one, the player's own generator: in range, and 0 for a max below 1.
        let mut own = AvmRng::default();
        assert!((0..4).contains(&own.avm1_random_action(4.0, &locale)));
        assert_eq!(own.avm1_random_action(0.0, &locale), 0);
        assert!((0.0..1.0).contains(&own.avm1_math_random(&locale)));
    }
}
