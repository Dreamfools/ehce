use crate::combat::CombatInputs;
use crate::combat::signals::inputs::update_player_behavior;
use bevy::app::{App, Plugin};
use bevy::ecs::query::QueryData;
use bevy::math::{FloatOrd, Vec2};
use bevy::prelude::{Bundle, Component, IntoScheduleConfigs as _, Query, Reflect};
use std::collections::hash_map::Entry;
use std::hash::Hash;
use std::ops::Add;
use utils::map::{HashMap, HashSet};

pub mod inputs;

pub struct SignalsPlugin;

impl Plugin for SignalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            CombatInputs,
            (update_player_behavior, sys_progress_signals).chain(),
        );
    }
}

fn sys_progress_signals(signals: Query<&mut UnitSignalsData>) {
    for mut signals in signals {
        signals.tick();
    }
}

#[derive(QueryData)]
pub struct UnitSignals {
    data: &'static UnitSignalsData,
}

impl UnitSignals {
    #[must_use]
    pub fn bundle() -> impl Bundle {
        UnitSignalsData::default()
    }
}

impl UnitSignalsItem<'_, '_> {
    /// Gets the value of a signal, or `Off` if it is not set
    #[must_use]
    pub fn get(&self, id: SignalId) -> SignalValue {
        self.data
            .signals
            .get(&id)
            .copied()
            .unwrap_or(SignalValue::Off)
    }

    /// Checks if a signal was changed since the last tick
    #[must_use]
    pub fn is_changed(&self, id: &SignalId) -> bool {
        self.data.changed.contains(id)
    }

    /// Checks if a signal was `Off` the last tick and is now set
    #[must_use]
    pub fn is_rising_edge(&self, id: &SignalId) -> bool {
        self.data.rising_edge.contains(id)
    }

    /// Checks if a signal was set the last tick and is now `Off`
    #[must_use]
    pub fn is_falling_edge(&self, id: &SignalId) -> bool {
        self.data.falling_edge.contains(id)
    }
}

#[derive(Debug, Clone, Default, Reflect, Component)]
struct UnitSignalsData {
    signals: HashMap<SignalId, SignalValue>,
    next: HashMap<SignalId, SignalValue>,

    buf: HashMap<SignalId, SignalValue>,

    rising_edge: HashSet<SignalId>,
    falling_edge: HashSet<SignalId>,
    changed: HashSet<SignalId>,
}

impl UnitSignalsData {
    /// Adds the signal to the next set of signals. Active signals should be added every frame
    fn add_next(&mut self, id: SignalId, value: SignalValue) {
        if value.is_zero() {
            return;
        }
        match self.next.entry(id) {
            Entry::Occupied(mut e) => {
                let sum = e.get() + &value;
                e.insert(sum);
            }
            Entry::Vacant(e) => {
                e.insert(value);
            }
        }
    }

    /// Clears the edge and changed sets. Should be called at the end of each tick
    fn tick(&mut self) {
        self.rising_edge.clear();
        self.falling_edge.clear();
        self.changed.clear();

        debug_assert!(self.buf.is_empty());

        for (id, value) in self.next.drain() {
            if value.is_zero() {
                continue;
            }
            let old_value = self.signals.remove(&id).unwrap_or(SignalValue::Off);

            if old_value != value {
                self.changed.insert(id);
                if old_value.is_zero() {
                    self.rising_edge.insert(id);
                }
            }

            self.buf.insert(id, value);
        }

        for (id, value) in self.signals.drain() {
            if value.is_zero() {
                continue;
            }
            self.falling_edge.insert(id);
        }

        std::mem::swap(&mut self.signals, &mut self.buf);

        debug_assert!(self.buf.is_empty());
        debug_assert!(self.next.is_empty());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum SignalId {
    /// Custom indexed action, usually for sitgnal or weapon activation
    Action(u16),
    /// Desired facing direction in local space
    Facing,
    /// Desired movement direction in local space
    Movement,
}

#[derive(Debug, Clone, Copy, Default, Reflect)]
pub enum SignalValue {
    /// Empty (unset) signal
    #[default]
    Off,
    /// Scalar signal value
    Scalar(f32),
    /// Vector signal value in local space
    Vector(Vec2),
}

impl SignalValue {
    #[must_use]
    pub fn new_vector(x: f32, y: f32) -> Self {
        Self::Vector(Vec2::new(x, y))
    }

    /// Checks if the signal value is zero. `Off` is considered zero, `Scalar`
    /// is zero if its value is zero, and `Vector` is zero if both components
    /// are zero
    #[must_use]
    pub fn is_zero(&self) -> bool {
        match self {
            SignalValue::Off => true,
            SignalValue::Scalar(a) => FloatOrd(*a) == FloatOrd(0.0),
            SignalValue::Vector(a) => {
                FloatOrd(a.x) == FloatOrd(0.0) && FloatOrd(a.y) == FloatOrd(0.0)
            }
        }
    }

    /// Converts the signal value to a scalar
    ///
    /// - `Off` is 0
    /// - `Scalar` is its value
    /// - `Vector` is its length
    #[must_use]
    pub fn as_scalar(&self) -> f32 {
        match self {
            SignalValue::Off => 0.0,
            SignalValue::Scalar(a) => *a,
            SignalValue::Vector(vec) => vec.length(),
        }
    }

    /// Converts the signal value to a vector
    ///
    /// - `Off` is (0, 0)
    /// - `Scalar` is (value, 0)
    /// - `Vector` is its value
    #[must_use]
    pub fn as_vector(&self) -> Vec2 {
        match self {
            SignalValue::Off => Vec2::ZERO,
            SignalValue::Scalar(a) => Vec2::new(*a, 0.0),
            SignalValue::Vector(vec) => *vec,
        }
    }
}

impl PartialEq for SignalValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SignalValue::Off, SignalValue::Off) => true,
            (SignalValue::Scalar(a), SignalValue::Scalar(b)) => FloatOrd(*a) == FloatOrd(*b),
            (SignalValue::Vector(a), SignalValue::Vector(b)) => {
                FloatOrd(a.x) == FloatOrd(b.x) && FloatOrd(a.y) == FloatOrd(b.y)
            }
            _ => false,
        }
    }
}

impl Eq for SignalValue {}

impl Hash for SignalValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            SignalValue::Off => 0.hash(state),
            SignalValue::Scalar(a) => {
                1.hash(state);
                FloatOrd(*a).hash(state);
            }
            SignalValue::Vector(a) => {
                2.hash(state);
                FloatOrd(a.x).hash(state);
                FloatOrd(a.y).hash(state);
            }
        }
    }
}

/// Defines addition for `SignalValue`. The rules are as follows:
/// - `Off` + `X` = `X`
/// - `Vector` + `X` = `Vector` + X.as_vector()
/// - `Scalar` + `Scalar` = `Scalar` with the sum of the values
impl Add<&SignalValue> for &SignalValue {
    type Output = SignalValue;

    fn add(self, rhs: &SignalValue) -> Self::Output {
        match (self, rhs) {
            (SignalValue::Off, rhs) => *rhs,
            (lhs, SignalValue::Off) => *lhs,
            (SignalValue::Vector(vec), rhs) => SignalValue::Vector(vec + rhs.as_vector()),
            (lhs, SignalValue::Vector(vec)) => SignalValue::Vector(lhs.as_vector() + vec),
            (SignalValue::Scalar(a), SignalValue::Scalar(b)) => SignalValue::Scalar(a + b),
        }
    }
}
