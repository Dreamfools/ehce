use crate::dyn_event;

const STEPS_PER_SECOND: u64 = 3600;
const ERROR_MARGIN: f64 = 0.05;

pub const FPS_30: Fps = Fps::new(30);
pub const FPS_60: Fps = Fps::new(60);
pub const FPS_75: Fps = Fps::new(75);
pub const FPS_120: Fps = Fps::new(120);
pub const FPS_144: Fps = Fps::new(144);
pub const FPS_240: Fps = Fps::new(240);

#[derive(Debug, Copy, Clone)]
pub struct Fps {
    steps: u64,
    timing: f64,
}

impl Fps {
    #[inline]
    #[must_use]
    const fn new(framerate: u64) -> Self {
        Self {
            steps: STEPS_PER_SECOND / framerate,
            timing: 1.0 / framerate as f64,
        }
    }

    #[inline]
    #[must_use]
    fn approx(&self, dt: f64, margin_percentage: f64) -> bool {
        let margin = self.timing * margin_percentage;
        let min = self.timing - margin;
        let max = self.timing + margin;
        dt >= min && dt <= max
    }
}

#[derive(Debug)]
pub struct Stepper {
    max_pending_updates: usize,
    target_fps: Fps,

    steps: u64,
    pending_updates: usize,
    total_updates: usize,

    // debug info
    frame_brackets: Vec<u64>,
}

impl Stepper {
    #[inline]
    #[must_use]
    pub fn new(max_pending_updates: usize, target_fps: Fps) -> Self {
        Self {
            max_pending_updates,
            target_fps,
            steps: 0,
            pending_updates: 0,
            total_updates: 0,
            frame_brackets: vec![0; max_pending_updates + 1],
        }
    }

    #[inline]
    #[must_use]
    pub fn dt(&self) -> f64 {
        self.target_fps.timing
    }

    /// Returns the fixed time of the current update
    ///
    /// This value is updated every time when call to [`Stepper::elapsed`]
    /// returns true
    #[inline]
    #[must_use]
    pub fn elapsed_time(&self) -> f64 {
        self.total_updates as f64 * self.target_fps.timing
    }

    /// Returns the number of updates that have been performed
    ///
    /// This value is updated every time when call to [`Stepper::elapsed`]
    /// returns true
    #[inline]
    #[must_use]
    pub fn elapsed_updates(&self) -> usize {
        self.total_updates
    }

    /// Informs the stepper of the elapsed time since the last frame
    ///
    /// Intended to be called once per frame
    pub fn elapsed(&mut self, dt: f64) {
        let mut rounded = false;
        for fps in [FPS_30, FPS_60, FPS_75, FPS_120, FPS_144, FPS_240] {
            if fps.approx(dt, ERROR_MARGIN) {
                self.steps += fps.steps;
                rounded = true;
                break;
            }
        }

        if !rounded {
            // If the time is not close to any of the FPS values, just use whatever is available

            self.steps += (dt * STEPS_PER_SECOND as f64).round() as u64;
        }

        while self.steps >= self.target_fps.steps {
            self.steps -= self.target_fps.steps;
            self.pending_updates += 1;
        }

        if self.pending_updates > self.max_pending_updates {
            self.pending_updates = self.max_pending_updates;
        }

        self.frame_brackets[self.pending_updates] += 1;

        // tracing::trace!(
        //     rounded,
        //     dt,
        //     steps = self.steps,
        //     pending_updates = self.pending_updates,
        //     "Fixed step"
        // );
    }

    /// Returns true if an update step should be performed
    ///
    /// This method will return true at most `max_full_frames` times per frame
    #[must_use]
    pub fn update(&mut self) -> bool {
        if self.pending_updates > 0 {
            self.pending_updates -= 1;
            self.total_updates += 1;
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn debug(&self) -> StepperDebug {
        StepperDebug::new(&self.frame_brackets)
    }
}

#[derive(Debug)]
pub struct StepperDebug {
    pub frame_brackets: Vec<u64>,
    pub skips_frequency: f64,
    pub double_frame_frequency: f64,
}

impl StepperDebug {
    #[must_use]
    pub fn new(frame_brackets: &[u64]) -> Self {
        let mut skips_count = 0u64;
        let mut double_count = 0u64;
        let mut total = 0u64;
        for (i, count) in frame_brackets.iter().copied().enumerate() {
            total += count;
            if i == 0 {
                skips_count += count;
            } else if i > 1 {
                double_count += count;
            }
        }
        Self {
            frame_brackets: frame_brackets.to_vec(),
            skips_frequency: skips_count as f64 / total as f64,
            double_frame_frequency: double_count as f64 / total as f64,
        }
    }

    #[inline]
    pub fn report(&self, level: tracing::Level) {
        dyn_event!(
            level,
            skips_frequency = self.skips_frequency,
            double_frame_frequency = self.double_frame_frequency,
            "stepper debug info"
        );
    }
}
