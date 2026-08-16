//! Deterministic 2D/3D improved Perlin and fBm noise helpers.
//! These are pure functions with no RNG state.

#[inline]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

#[inline]
fn hash2(ix: i64, iy: i64, seed: u64) -> u64 {
    let mut x = (ix as u64).wrapping_mul(0x9E3779B97F4A7C15);
    x ^= (iy as u64).wrapping_mul(0xC2B2AE3D27D4EB4F);
    x ^= seed.wrapping_mul(0x165667B19E3779F9);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

#[inline]
fn hash3(ix: i64, iy: i64, iz: i64, seed: u64) -> u64 {
    let mut x = (ix as u64).wrapping_mul(0x9E3779B97F4A7C15);
    x ^= (iy as u64).wrapping_mul(0xC2B2AE3D27D4EB4F);
    x ^= (iz as u64).wrapping_mul(0x165667B19E3779F9);
    x ^= seed.wrapping_mul(0xD6E8FEB86659FD93);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

#[inline]
fn grad(hash: u64, x: f64, y: f64) -> f64 {
    match hash & 7 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x,
        5 => -x,
        6 => y,
        _ => -y,
    }
}

#[inline]
fn grad3(hash: u64, x: f64, y: f64, z: f64) -> f64 {
    // The twelve edge directions are the classic improved-Perlin gradient set.
    let value = match hash % 12 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        _ => -y - z,
    };
    value * 0.7071067811865475
}

/// Classic gradient Perlin noise in 2D, output approximately in [-1, 1].
pub fn perlin2(x: f64, y: f64, seed: u64) -> f64 {
    let x0 = x.floor() as i64;
    let y0 = y.floor() as i64;
    let xf = x - x0 as f64;
    let yf = y - y0 as f64;

    let h00 = hash2(x0, y0, seed);
    let h10 = hash2(x0 + 1, y0, seed);
    let h01 = hash2(x0, y0 + 1, seed);
    let h11 = hash2(x0 + 1, y0 + 1, seed);

    let n00 = grad(h00, xf, yf);
    let n10 = grad(h10, xf - 1.0, yf);
    let n01 = grad(h01, xf, yf - 1.0);
    let n11 = grad(h11, xf - 1.0, yf - 1.0);

    let u = fade(xf);
    let v = fade(yf);
    let nx0 = lerp(n00, n10, u);
    let nx1 = lerp(n01, n11, u);
    (lerp(nx0, nx1, v) * 0.7071067811865475).clamp(-1.0, 1.0)
}

/// Classic improved gradient Perlin noise in 3D, output approximately in [-1, 1].
pub fn perlin3(x: f64, y: f64, z: f64, seed: u64) -> f64 {
    let x0 = x.floor() as i64;
    let y0 = y.floor() as i64;
    let z0 = z.floor() as i64;
    let xf = x - x0 as f64;
    let yf = y - y0 as f64;
    let zf = z - z0 as f64;

    let n000 = grad3(hash3(x0, y0, z0, seed), xf, yf, zf);
    let n100 = grad3(hash3(x0 + 1, y0, z0, seed), xf - 1.0, yf, zf);
    let n010 = grad3(hash3(x0, y0 + 1, z0, seed), xf, yf - 1.0, zf);
    let n110 = grad3(hash3(x0 + 1, y0 + 1, z0, seed), xf - 1.0, yf - 1.0, zf);
    let n001 = grad3(hash3(x0, y0, z0 + 1, seed), xf, yf, zf - 1.0);
    let n101 = grad3(hash3(x0 + 1, y0, z0 + 1, seed), xf - 1.0, yf, zf - 1.0);
    let n011 = grad3(hash3(x0, y0 + 1, z0 + 1, seed), xf, yf - 1.0, zf - 1.0);
    let n111 = grad3(
        hash3(x0 + 1, y0 + 1, z0 + 1, seed),
        xf - 1.0,
        yf - 1.0,
        zf - 1.0,
    );

    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);

    let nx00 = lerp(n000, n100, u);
    let nx10 = lerp(n010, n110, u);
    let nx01 = lerp(n001, n101, u);
    let nx11 = lerp(n011, n111, u);
    let nxy0 = lerp(nx00, nx10, v);
    let nxy1 = lerp(nx01, nx11, v);

    lerp(nxy0, nxy1, w).clamp(-1.0, 1.0)
}

/// Fractal Brownian Motion using Perlin noise, normalized to roughly [-1, 1].
pub fn fbm2(x: f64, y: f64, seed: u64, octaves: u32, persistence: f64, lacunarity: f64) -> f64 {
    let oct = octaves.max(1);
    let mut frequency = 1.0;
    let mut amplitude = 1.0;
    let mut sum = 0.0;
    let mut amp_sum = 0.0;

    for octave in 0..oct {
        let octave_seed = seed.wrapping_add((octave as u64).wrapping_mul(0x9E3779B97F4A7C15));
        sum += amplitude * perlin2(x * frequency, y * frequency, octave_seed);
        amp_sum += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }

    if amp_sum > 0.0 {
        (sum / amp_sum).clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

/// Fractal Brownian Motion using 3D Perlin noise, normalized to roughly [-1, 1].
pub fn fbm3(
    x: f64,
    y: f64,
    z: f64,
    seed: u64,
    octaves: u32,
    persistence: f64,
    lacunarity: f64,
) -> f64 {
    let oct = octaves.max(1);
    let mut frequency = 1.0;
    let mut amplitude = 1.0;
    let mut sum = 0.0;
    let mut amp_sum = 0.0;

    for octave in 0..oct {
        let octave_seed = seed.wrapping_add((octave as u64).wrapping_mul(0x9E3779B97F4A7C15));
        sum += amplitude * perlin3(x * frequency, y * frequency, z * frequency, octave_seed);
        amp_sum += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }

    if amp_sum > 0.0 {
        (sum / amp_sum).clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

/// Turbulence built from the absolute value of 3D Perlin octaves, normalized to [0, 1].
pub fn turbulence3(
    x: f64,
    y: f64,
    z: f64,
    seed: u64,
    octaves: u32,
    persistence: f64,
    lacunarity: f64,
) -> f64 {
    let oct = octaves.max(1);
    let mut frequency = 1.0;
    let mut amplitude = 1.0;
    let mut sum = 0.0;
    let mut amp_sum = 0.0;

    for octave in 0..oct {
        let octave_seed = seed.wrapping_add((octave as u64).wrapping_mul(0x9E3779B97F4A7C15));
        sum += amplitude * perlin3(x * frequency, y * frequency, z * frequency, octave_seed).abs();
        amp_sum += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }

    if amp_sum > 0.0 {
        (sum / amp_sum).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perlin2_is_deterministic() {
        let a = perlin2(1.234, -5.678, 42);
        let b = perlin2(1.234, -5.678, 42);
        assert!((a - b).abs() < 1e-12);
    }

    #[test]
    fn test_fbm2_stays_in_range() {
        let v = fbm2(2.75, -1.5, 7, 5, 0.5, 2.0);
        assert!((-1.0..=1.0).contains(&v));
    }

    #[test]
    fn test_perlin3_is_deterministic() {
        let a = perlin3(1.234, -5.678, 9.1011, 42);
        let b = perlin3(1.234, -5.678, 9.1011, 42);
        assert!((a - b).abs() < 1e-12);
    }

    #[test]
    fn test_perlin3_is_continuous_at_lattice_boundary() {
        let left = perlin3(1.0 - 1e-7, 0.3, -0.2, 7);
        let right = perlin3(1.0 + 1e-7, 0.3, -0.2, 7);
        assert!((left - right).abs() < 1e-5);
    }

    #[test]
    fn test_turbulence3_stays_in_range() {
        let v = turbulence3(2.75, -1.5, 0.25, 7, 5, 0.5, 2.0);
        assert!((0.0..=1.0).contains(&v));
    }
}
