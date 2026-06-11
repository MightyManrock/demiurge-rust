use image::{ImageBuffer, Rgb};
use noise::{Fbm, NoiseFn, Perlin};

/// A 2D float field over a normalized [0,1) x [0,1) coordinate plane.
/// x wraps (east-west); y clamps (poles do not connect).
struct HeatMap {
    width: usize,
    height: usize,
    data: Vec<f64>,
}

impl HeatMap {
    fn generate_elevation(width: usize, height: usize, seed: u32) -> Self {
        let fbm = Fbm::<Perlin>::new(seed);

        // Fill raw noise values.
        let mut data = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let nx = x as f64 / width as f64 * 3.5;
                let ny = y as f64 / height as f64 * 2.0;
                data.push(fbm.get([nx, ny]));
            }
        }

        // Normalize to [0, 1] using actual min/max so the full color ramp is used.
        let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;
        for v in &mut data {
            *v = (*v - min) / range;
        }

        HeatMap { width, height, data }
    }

    /// Sample by bilinear interpolation. x and y are in [0, 1).
    fn sample(&self, x: f64, y: f64) -> f64 {
        let px = x.rem_euclid(1.0) * self.width as f64;
        let py = y.clamp(0.0, 1.0 - f64::EPSILON) * self.height as f64;

        let x0 = px.floor() as usize % self.width;
        let y0 = py.floor() as usize;
        let x1 = (x0 + 1) % self.width;
        let y1 = (y0 + 1).min(self.height - 1);

        let tx = px.fract();
        let ty = py.fract();

        let v00 = self.data[y0 * self.width + x0];
        let v10 = self.data[y0 * self.width + x1];
        let v01 = self.data[y1 * self.width + x0];
        let v11 = self.data[y1 * self.width + x1];

        let top = v00 + (v10 - v00) * tx;
        let bot = v01 + (v11 - v01) * tx;
        top + (bot - top) * ty
    }
}

fn lerp_color(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    [
        (a[0] as f64 + (b[0] as f64 - a[0] as f64) * t).round() as u8,
        (a[1] as f64 + (b[1] as f64 - a[1] as f64) * t).round() as u8,
        (a[2] as f64 + (b[2] as f64 - a[2] as f64) * t).round() as u8,
    ]
}

/// Raw elevation gradient: red (lowest) → yellow (mid) → green (highest).
/// No terrain semantics — ocean level, biomes, and rivers are separate concerns.
fn elevation_color(t: f64) -> [u8; 3] {
    const STOPS: &[([u8; 3], f64)] = &[
        ([255,   0,   0], 0.00), // lowest
        ([255, 255,   0], 0.50), // mid
        ([  0, 255,   0], 1.00), // highest
    ];

    for i in 0..STOPS.len() - 1 {
        let (color_a, t_a) = STOPS[i];
        let (color_b, t_b) = STOPS[i + 1];
        if t <= t_b {
            let local_t = (t - t_a) / (t_b - t_a);
            return lerp_color(color_a, color_b, local_t.clamp(0.0, 1.0));
        }
    }
    STOPS.last().unwrap().0
}

fn main() {
    let width = 1024usize;
    let height = 512usize;
    let seed = 42u32;
    let output = "elevation.png";

    println!("Generating {}x{} elevation map (seed {})...", width, height, seed);
    let map = HeatMap::generate_elevation(width, height, seed);

    println!("Rendering...");
    let img = ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let nx = x as f64 / width as f64;
        let ny = y as f64 / height as f64;
        let value = map.sample(nx, ny);
        Rgb(elevation_color(value))
    });

    img.save(output).expect("failed to save image");
    println!("Saved {}", output);
}
