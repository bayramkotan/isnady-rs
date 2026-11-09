//! Python's `random.Random(seed)` (the Mersenne Twister MT19937 as CPython seeds it): the statistics draw their
//! bootstrap samples with a fixed seed, so the Rust isnady draws the very same samples and gives the same intervals.

const N: usize = 624;
const M: usize = 397;

pub struct Random {
    mt: [u32; N],
    index: usize,
}

impl Random {
    /// `random.Random(seed)` for a non-negative integer seed below 2³².
    pub fn new(seed: u32) -> Random {
        let mut r = Random {
            mt: [0; N],
            index: N,
        };
        r.init_genrand(19650218);
        // init_by_array with the key [seed] (CPython splits abs(seed) into 32-bit words)
        let key = [seed];
        let (mut i, mut j) = (1usize, 0usize);
        let mut k = N.max(key.len());
        while k > 0 {
            let prev = r.mt[i - 1] ^ (r.mt[i - 1] >> 30);
            r.mt[i] = (r.mt[i] ^ prev.wrapping_mul(1664525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                r.mt[0] = r.mt[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
            k -= 1;
        }
        k = N - 1;
        while k > 0 {
            let prev = r.mt[i - 1] ^ (r.mt[i - 1] >> 30);
            r.mt[i] = (r.mt[i] ^ prev.wrapping_mul(1566083941)).wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                r.mt[0] = r.mt[N - 1];
                i = 1;
            }
            k -= 1;
        }
        r.mt[0] = 0x8000_0000;
        r.index = N;
        r
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..N {
            self.mt[i] = 1812433253u32
                .wrapping_mul(self.mt[i - 1] ^ (self.mt[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        self.index = N;
    }

    fn next_u32(&mut self) -> u32 {
        const MAG: [u32; 2] = [0, 0x9908_b0df];
        if self.index >= N {
            for kk in 0..N {
                let y = (self.mt[kk] & 0x8000_0000) | (self.mt[(kk + 1) % N] & 0x7fff_ffff);
                self.mt[kk] = self.mt[(kk + M) % N] ^ (y >> 1) ^ MAG[(y & 1) as usize];
            }
            self.index = 0;
        }
        let mut y = self.mt[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// `random()`: a float in [0, 1) with 53 random bits.
    pub fn random(&mut self) -> f64 {
        let a = (self.next_u32() >> 5) as f64;
        let b = (self.next_u32() >> 6) as f64;
        (a * 67108864.0 + b) * (1.0 / 9007199254740992.0)
    }

    /// `getrandbits(k)` for 0 < k ≤ 32.
    fn bits(&mut self, k: u32) -> u32 {
        self.next_u32() >> (32 - k)
    }

    /// `randrange(n)`: an integer in [0, n), drawn as CPython's `_randbelow_with_getrandbits` draws it.
    pub fn randrange(&mut self, n: usize) -> usize {
        let k = usize::BITS - n.leading_zeros();
        if k > 32 {
            // never needed here: samples are far below 2³² cases
            let hi = self.bits(32) as u64;
            return (hi as usize) % n;
        }
        let mut r = self.bits(k) as usize;
        while r >= n {
            r = self.bits(k) as usize;
        }
        r
    }

    /// The index `random.choices(population, k=…)` takes for each draw: floor(random() · n).
    pub fn choice_index(&mut self, n: usize) -> usize {
        (self.random() * n as f64).floor() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::Random;
    use crate::stats::num::{erfc, fsum, lgamma};

    #[test]
    fn as_python_draws() {
        // random.Random(1405).random(), then .randrange(3517)
        let mut r = Random::new(1405);
        assert_eq!(r.random(), 0.4225459607447182);
        let mut r = Random::new(1405);
        r.random();
        assert_eq!(r.randrange(3517), 983);
    }

    #[test]
    fn as_python_math() {
        assert_eq!(fsum([0.1; 10]), 1.0);
        assert_eq!(lgamma(2.5), 0.2846828704729196);
        // erfc is the C library's, as in CPython: the last bit may differ from one platform's to another's (macOS)
        assert!((erfc(0.7) - 0.32219880616258156).abs() <= 2.0 * f64::EPSILON * 0.33);
    }
}
