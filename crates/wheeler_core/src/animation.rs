//! Port of `Animation/TimeFloatInterpolator` and `TimeBounceInterpolator`.
//!
//! The C++ interpolators register themselves in a global
//! `TimeFloatInterpolatorManager` ticked once per frame, and run completion
//! callbacks on detached threads. Here each owner ticks its interpolators with
//! [`FloatInterpolator::update`] once per frame instead, and the bounce
//! "callback" is handled inline; the visible behaviour is the same.

/// C++ `TimeFloatInterpolator`
#[derive(Debug, Clone, Default)]
pub struct FloatInterpolator {
    value: f64,
    target: f64,
    duration: f64,
    elapsed: f64,
}

impl FloatInterpolator {
    pub const fn new(initial: f64) -> Self {
        Self {
            value: initial,
            target: initial,
            duration: 0.0,
            elapsed: 0.0,
        }
    }

    /// C++ `InterpolateTo`
    pub fn interpolate_to(&mut self, target: f64, duration: f64) {
        self.target = target;
        self.duration = duration;
        self.elapsed = 0.0;
    }

    /// C++ `Update`: advances by `dt`; returns `true` on the frame the
    /// interpolation finishes.
    pub fn update(&mut self, dt: f64) -> bool {
        if self.elapsed < self.duration {
            self.elapsed += dt;
            let t = (self.elapsed / self.duration).min(1.0);
            // Same easing as the original: move `t` of the remaining distance.
            self.value += (self.target - self.value) * t;
            if self.elapsed >= self.duration {
                return true;
            }
        }
        false
    }

    pub const fn value(&self) -> f64 {
        self.value
    }

    /// C++ `ForceFinish`
    pub fn force_finish(&mut self) {
        self.elapsed = self.duration;
        self.value = self.target;
    }

    /// C++ `SetValue`
    pub fn set_value(&mut self, value: f64) {
        self.value = value;
    }

    /// C++ `ForceValue`
    pub fn force_value(&mut self, value: f64) {
        self.target = value;
        self.value = value;
        self.duration = 0.0;
        self.elapsed = 0.0;
    }
}

/// C++ `TimeBounceInterpolator`: goes to a target, then back to the original
/// value over the same duration.
#[derive(Debug, Clone)]
pub struct BounceInterpolator {
    interpolator: FloatInterpolator,
    original: f64,
    duration: f64,
    returning: bool,
}

impl BounceInterpolator {
    pub const fn new(original: f64) -> Self {
        Self {
            interpolator: FloatInterpolator::new(original),
            original,
            duration: 0.0,
            returning: false,
        }
    }

    pub fn interpolate_to(&mut self, target: f64, duration: f64) {
        self.duration = duration;
        self.returning = false;
        self.interpolator.interpolate_to(target, duration);
    }

    pub fn update(&mut self, dt: f64) {
        if self.interpolator.update(dt) && !self.returning {
            self.returning = true;
            self.interpolator
                .interpolate_to(self.original, self.duration);
        }
    }

    pub const fn value(&self) -> f64 {
        self.interpolator.value()
    }

    /// C++ `ForceFinish`: jumps back to the original value.
    pub fn force_finish(&mut self) {
        self.interpolator.force_finish();
        if !self.returning {
            self.interpolator.force_value(self.original);
        }
        self.returning = false;
    }

    pub fn set_value(&mut self, value: f64) {
        self.interpolator.set_value(value);
    }

    pub fn force_value(&mut self, value: f64) {
        self.interpolator.force_value(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_reaches_target() {
        let mut i = FloatInterpolator::new(0.0);
        i.interpolate_to(10.0, 0.2);
        let mut finished = false;
        for _ in 0..10 {
            finished |= i.update(0.05);
        }
        assert!(finished);
        assert!((i.value() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn bounce_returns_to_original() {
        let mut b = BounceInterpolator::new(0.0);
        b.interpolate_to(-5.0, 0.1);
        for _ in 0..10 {
            b.update(0.05);
        }
        assert!(b.value().abs() < 1e-9);
    }
}
