use std::ops::Add as _;
use std::time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
pub type StaggererImpl = TimedStaggerer;

#[cfg(target_arch = "wasm32")]
pub type StaggererImpl = StaggererDummy;

pub trait Staggerer {
    fn new(delay: f64, min_interval: f64) -> Self;
    /// Emits trigger to staggerer
    fn trigger(&mut self);

    /// Checks if staggerer is emitting activation signal, consuming it if true
    fn activated(&mut self) -> bool;
}

#[derive(Debug)]
pub struct TimedStaggerer {
    /// Stagger delay before activation after signal is received
    pub delay: Duration,
    /// Minimal interval between activations
    pub min_interval: Duration,

    /// Last emitted activation
    earliest_next_activation: Instant,
    /// Last received trigger
    last_trigger: Option<Instant>,
}

impl Staggerer for TimedStaggerer {
    fn new(delay: f64, min_interval: f64) -> Self {
        Self {
            delay: Duration::from_millis((delay * 1000.0).round() as u64),
            min_interval: Duration::from_millis((min_interval * 1000.0).round() as u64),
            earliest_next_activation: Instant::now(),
            last_trigger: None,
        }
    }

    fn trigger(&mut self) {
        self.last_trigger = Some(Instant::now());
    }

    fn activated(&mut self) -> bool {
        if Instant::now() < self.earliest_next_activation {
            return false;
        }

        let Some(last_trigger) = &self.last_trigger else {
            return false;
        };

        if Instant::now() > last_trigger.add(self.delay) {
            self.last_trigger = None;
            self.earliest_next_activation = Instant::now() + self.min_interval;
            true
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub struct StaggererDummy {
    activated: bool,
}

impl Staggerer for StaggererDummy {
    fn new(_delay: f64, _min_interval: f64) -> Self {
        Self { activated: false }
    }

    fn trigger(&mut self) {
        self.activated = true;
    }

    fn activated(&mut self) -> bool {
        if self.activated {
            self.activated = false;
            true
        } else {
            false
        }
    }
}
