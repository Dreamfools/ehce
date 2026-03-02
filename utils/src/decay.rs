use std::ops::{Add, Mul, Sub};

pub fn exp_decay<N>(a: N, b: N, decay: f32, dt: f32) -> N
where
    N: Copy + Add<Output = N> + Sub<Output = N> + Mul<f32, Output = N>,
{
    b + (a - b) * (-decay * dt).exp()
}
