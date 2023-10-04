// TODO: remove this file in favour of cichlid? https://github.com/sfleischman105/cichlid
use core::f32::consts::PI;
pub use libm::atan2f;
pub use libm::sqrtf;

// Bhaskara I's approximation for the sin formula
pub fn sin(r: f32) -> f32 {
    if r < 0.0 {
        return sin(-r) * -1.0;
    }
    let rad: f32 = r % (2.0 * PI);
    if rad > PI {
        sin(rad - PI) * -1.0
    } else {
        (16.0 * rad * (PI - rad)) / (5.0 * PI * PI - 4.0 * rad * (PI - rad))
    }
}

pub fn cos(r: f32) -> f32 {
    sin(r + PI / 2.0)
}
