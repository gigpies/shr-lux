//! Cheap source-clock activity cues. Missing PCM is handled by the caller's reset,
//! never interpreted as a musical pause. These are heuristics, not song semantics.
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[default]
    Unknown,
    Steady,
    Up,
    Down,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    #[default]
    Settling,
    Playing,
    BriefGap,
    Pause,
    Silence,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Expression {
    /// Exclusive end of the 10 ms activity window (separate from spectral time).
    pub source_frame: u64,
    pub activity: Activity,
    pub quiet_ms: u32,
    pub resumed: bool,
    pub energy_direction: Direction,
    /// Spectral register motion, not melody or circle-of-fifths direction.
    pub register_direction: Direction,
}
#[derive(Clone, Default)]
pub(crate) struct ExpressionTracker {
    blocks: u32,
    quiet: u32,
    resume: u8,
    reference: f32,
    fast: f32,
    slow: f32,
    register: Option<f32>,
    candidate: Direction,
    confirmations: u8,
    direction: Direction,
}
impl ExpressionTracker {
    pub fn process<const N: usize>(&mut self, frames: &[[f32; N]], end: u64) -> Expression {
        // Independent source powers avoid cancellation; max RMS means a lone
        // instrument prevents an ensemble pause regardless of configured count.
        let mut powers = [0.0_f32; N];
        for frame in frames {
            for (p, sample) in powers.iter_mut().zip(frame) {
                *p += sample * sample;
            }
        }
        let energy = (powers.iter().sum::<f32>() / (N * frames.len()) as f32).sqrt();
        let level =
            powers.into_iter().fold(0.0_f32, f32::max).sqrt() / (frames.len() as f32).sqrt();
        self.blocks = self.blocks.saturating_add(1);
        self.reference = level.max(self.reference * 0.999);
        let threshold = (self.reference * 0.02).max(0.0001);
        if level < threshold {
            self.quiet = self.quiet.saturating_add(10).min(u32::MAX / 10 * 10);
            self.resume = 0;
        } else {
            if self.quiet >= 350 {
                self.resume = 10;
            } else {
                self.resume = self.resume.saturating_sub(1);
            }
            self.quiet = 0;
        }
        if self.blocks == 1 {
            self.fast = energy;
            self.slow = energy;
        }
        self.fast += 0.04 * (energy - self.fast);
        self.slow += 0.005 * (energy - self.slow);
        let energy_direction = if self.blocks < 50 || self.quiet >= 120 {
            Direction::Steady
        } else if self.fast > self.slow * 1.25 {
            Direction::Up
        } else if self.fast < self.slow * 0.8 {
            Direction::Down
        } else {
            Direction::Steady
        };
        if self.quiet >= 120 {
            self.register = None;
            self.direction = Direction::Unknown;
            self.confirmations = 0;
        }
        Expression {
            source_frame: end,
            activity: if self.blocks < 12 {
                Activity::Settling
            } else if self.quiet >= 3000 {
                Activity::Silence
            } else if self.quiet >= 350 {
                Activity::Pause
            } else if self.quiet >= 120 {
                Activity::BriefGap
            } else {
                Activity::Playing
            },
            quiet_ms: self.quiet,
            resumed: self.resume > 0,
            energy_direction,
            register_direction: self.direction,
        }
    }
    pub fn register(&mut self, pitch: Option<f32>) -> Direction {
        let direction = match (self.register, pitch) {
            (Some(old), Some(new)) if new - old > 0.75 => Direction::Up,
            (Some(old), Some(new)) if old - new > 0.75 => Direction::Down,
            (Some(_), Some(_)) => Direction::Steady,
            _ => Direction::Unknown,
        };
        if direction == self.candidate {
            self.confirmations = self.confirmations.saturating_add(1);
        } else {
            self.candidate = direction;
            self.confirmations = 1;
        }
        self.direction = if self.confirmations >= 2 {
            direction
        } else {
            Direction::Unknown
        };
        self.register = match (self.register, pitch) {
            (Some(old), Some(new)) => Some(old + 0.3 * (new - old)),
            (_, value) => value,
        };
        self.direction
    }
}
