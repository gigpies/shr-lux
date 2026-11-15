//! Causal, bounded harmonic and arrangement estimates for isolated 48 kHz feeds.
//!
//! Integer pitch classes and Camelot labels describe relationships; they do not
//! replace spectral analysis or prove a song's key or verse/chorus semantics.
use crate::{
    analysis::WINDOW,
    expression::{Expression, ExpressionTracker},
};
use std::f32::consts::PI;

const FFT: usize = 2048;
const HOP: usize = FFT / 2;
const DECIMATION: usize = 6;
const TAPS: usize = 96;
const RATE: f32 = 8000.0;
const KEY_HISTORY: usize = 96;
const SECTION_HOPS: usize = 16; // 2.048 seconds, not a claimed musical bar.
const SECTION_MEMORY: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Major,
    Minor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChordEstimate {
    /// C=0, C#=1, ..., B=11. Enharmonic spelling is deliberately unspecified.
    pub root: u8,
    pub quality: Mode,
    pub confidence: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyEstimate {
    pub tonic: u8,
    pub mode: Mode,
    pub camelot_number: u8,
    pub camelot_letter: char,
    pub confidence: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementKind {
    Stationary,
    /// A root moves up a perfect fifth: C -> G.
    FifthsForward,
    /// A root moves down a perfect fifth: G -> C.
    FifthsBackward,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HarmonicMovement {
    /// Shortest signed circle displacement; positive means ascending fifths.
    /// The opposite point has the deterministic value +6.
    pub fifths_steps: i8,
    pub kind: MovementKind,
    /// Heuristic diatonic/root-function tension, not a quality judgment.
    pub tension: f32,
    /// Cadential support for dominant -> tonic in an independently known key.
    pub resolution: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionLabel {
    Unknown,
    VerseCandidate,
    ChorusCandidate,
    BridgeCandidate,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionEstimate {
    /// Locally assigned recurring arrangement identity, reset with the timeline.
    pub id: u32,
    pub novelty: f32,
    pub recurrence: f32,
    pub label: SectionLabel,
    pub confidence: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicalSnapshot {
    /// Exclusive source end of latest spectral window, up to 6143 frames behind
    /// the consumed source block end. Zero before readiness.
    pub source_frame: u64,
    /// A complete valid spectral window exists. Silence can be ready with no chord.
    pub ready: bool,
    pub chroma: [f32; 12],
    pub chord: Option<ChordEstimate>,
    pub key: Option<KeyEstimate>,
    pub movement: Option<HarmonicMovement>,
    pub section: Option<SectionEstimate>,
    pub expression: Expression,
    pub prediction: Option<ChordEstimate>,
}
impl Default for MusicalSnapshot {
    fn default() -> Self {
        Self {
            source_frame: 0,
            ready: false,
            chroma: [0.0; 12],
            chord: None,
            key: None,
            movement: None,
            section: None,
            expression: Expression::default(),
            prediction: None,
        }
    }
}

/// No allocation, locking, I/O or trigonometry in `process`. Constructor builds
/// the filter/window/twiddles once. This is an analysis worker, not a qualified
/// realtime sound-card callback: Pi CPU/latency acceptance remains separate.
pub type MusicalAnalyzer = SourceMusicalAnalyzer<4>;

/// Configured analysis capacity is a worker resource bound, not a product input cap.
pub const MAX_MUSIC_SOURCES: usize = 16;

#[derive(Clone)]
pub struct SourceMusicalAnalyzer<const N: usize> {
    weights: [f32; N],
    expression: ExpressionTracker,
    transitions: [[u16; 24]; 24],
    fir_coeff: [f32; TAPS],
    fir: [[f32; TAPS]; N],
    fir_cursor: usize,
    decimator: usize,
    ring: [[f32; FFT]; N],
    cursor: usize,
    filled: usize,
    since_hop: usize,
    window: [f32; FFT],
    twiddle_re: [f32; FFT / 2],
    twiddle_im: [f32; FFT / 2],
    real: [f32; FFT],
    imag: [f32; FFT],
    bin_note: [u8; FFT / 2],
    expected_frame: Option<u64>,
    snapshot: MusicalSnapshot,
    candidate: Option<ChordEstimate>,
    confirmations: u8,
    previous_chord: Option<ChordEstimate>,
    movement_age: u8,
    ambiguous_hops: u8,
    key_history: [[f32; 12]; KEY_HISTORY],
    key_cursor: usize,
    key_count: usize,
    key_roots: [u16; KEY_HISTORY],
    quiet_hops: u8,
    section_tracker: SectionTracker<N>,
    section_chroma: [f32; 12],
    section_levels: [f32; N],
    section_count: usize,
}
impl Default for SourceMusicalAnalyzer<4> {
    fn default() -> Self {
        Self::new()
    }
}
impl SourceMusicalAnalyzer<4> {
    pub fn new() -> Self {
        Self::with_weights([0.0, 0.35, 1.0, 1.0]).expect("legacy roles")
    }
}
impl<const N: usize> SourceMusicalAnalyzer<N> {
    pub fn with_weights(weights: [f32; N]) -> Result<Self, &'static str> {
        if N == 0
            || N > MAX_MUSIC_SOURCES
            || weights
                .iter()
                .any(|w| !w.is_finite() || !(0.0..=1.0).contains(w))
        {
            return Err("music source budget/weights");
        }
        // 96-tap Hann-windowed sinc, 2.5 kHz cutoff before 8 kHz decimation.
        // Pitch analysis stops below 2.1 kHz; this leaves an anti-alias guard band.
        let mut fir_coeff = [0.0; TAPS];
        let cutoff = 2500.0 / 48_000.0;
        for (n, c) in fir_coeff.iter_mut().enumerate() {
            let x = n as f32 - (TAPS - 1) as f32 * 0.5;
            *c = (2.0 * PI * cutoff * x).sin() / (PI * x)
                * (0.5 - 0.5 * (2.0 * PI * n as f32 / (TAPS - 1) as f32).cos());
        }
        let sum: f32 = fir_coeff.iter().sum();
        for c in &mut fir_coeff {
            *c /= sum;
        }
        let window =
            std::array::from_fn(|n| 0.5 - 0.5 * (2.0 * PI * n as f32 / (FFT - 1) as f32).cos());
        let twiddle_re = std::array::from_fn(|n| (2.0 * PI * n as f32 / FFT as f32).cos());
        let twiddle_im = std::array::from_fn(|n| -(2.0 * PI * n as f32 / FFT as f32).sin());
        let bin_note = std::array::from_fn(|bin| {
            let frequency = bin as f32 * RATE / FFT as f32;
            if !(63.0..=2100.0).contains(&frequency) {
                return u8::MAX;
            }
            let midi = (69.0 + 12.0 * (frequency / 440.0).log2()).round() as u8;
            if (36..=95).contains(&midi) {
                midi - 36
            } else {
                u8::MAX
            }
        });
        Ok(Self {
            weights,
            expression: ExpressionTracker::default(),
            transitions: [[0; 24]; 24],
            fir_coeff,
            fir: [[0.0; TAPS]; N],
            fir_cursor: 0,
            decimator: 0,
            ring: [[0.0; FFT]; N],
            cursor: 0,
            filled: 0,
            since_hop: 0,
            window,
            twiddle_re,
            twiddle_im,
            real: [0.0; FFT],
            imag: [0.0; FFT],
            bin_note,
            expected_frame: None,
            snapshot: MusicalSnapshot::default(),
            candidate: None,
            confirmations: 0,
            previous_chord: None,
            movement_age: 0,
            ambiguous_hops: 0,
            key_history: [[0.0; 12]; KEY_HISTORY],
            key_cursor: 0,
            key_count: 0,
            key_roots: [0; KEY_HISTORY],
            quiet_hops: 0,
            section_tracker: SectionTracker::default(),
            section_chroma: [0.0; 12],
            section_levels: [0.0; N],
            section_count: 0,
        })
    }

    /// Clear timeline-derived state while retaining precomputed coefficients.
    pub fn reset(&mut self) {
        self.expression = ExpressionTracker::default();
        self.transitions = [[0; 24]; 24];
        self.fir.fill([0.0; TAPS]);
        self.ring.fill([0.0; FFT]);
        self.fir_cursor = 0;
        self.decimator = 0;
        self.cursor = 0;
        self.filled = 0;
        self.since_hop = 0;
        self.expected_frame = None;
        self.snapshot = MusicalSnapshot::default();
        self.candidate = None;
        self.confirmations = 0;
        self.previous_chord = None;
        self.movement_age = 0;
        self.ambiguous_hops = 0;
        self.key_history.fill([0.0; 12]);
        self.key_cursor = 0;
        self.key_count = 0;
        self.key_roots.fill(0);
        self.quiet_hops = 0;
        self.section_tracker = SectionTracker::default();
        self.section_chroma = [0.0; 12];
        self.section_levels = [0.0; N];
        self.section_count = 0;
    }

    pub fn snapshot(&self) -> MusicalSnapshot {
        self.snapshot
    }

    /// Consume exactly one 10 ms block of normalized PCM, at its source clock
    /// position. A gap/backward jump resets before accepting the new timeline.
    /// Malformed PCM/block length/overflow or more than two ADC-clipped frames
    /// returns an unavailable reset snapshot and erases harmonic history.
    pub fn process(&mut self, frames: &[[f32; N]], first_frame: u64) -> MusicalSnapshot {
        let Some(end) = first_frame.checked_add(WINDOW as u64) else {
            self.reset();
            return self.snapshot;
        };
        if frames.len() != WINDOW
            || frames
                .iter()
                .flatten()
                .any(|x| !x.is_finite() || x.abs() > 1.0)
        {
            self.reset();
            return self.snapshot;
        }
        // Match the existing source reliability gate across all four feeds.
        // Never retain saturated ADC spectra in the longer key/section history.
        if frames
            .iter()
            .filter(|frame| frame.iter().any(|sample| sample.abs() >= 0.999))
            .take(3)
            .count()
            > 2
        {
            self.reset();
            return self.snapshot;
        }
        if self
            .expected_frame
            .is_some_and(|expected| expected != first_frame)
        {
            self.reset();
        }
        self.expected_frame = Some(end);
        self.snapshot.expression = self.expression.process(frames, end);
        for (offset, frame) in frames.iter().enumerate() {
            for (channel, sample) in self.fir.iter_mut().zip(frame) {
                channel[self.fir_cursor] = *sample;
            }
            self.fir_cursor = (self.fir_cursor + 1) % TAPS;
            self.decimator += 1;
            if self.decimator != DECIMATION {
                continue;
            }
            self.decimator = 0;
            for source in 0..N {
                if self.weights[source] == 0.0 {
                    continue;
                }
                let mut value = 0.0;
                for tap in 0..TAPS {
                    value += self.fir_coeff[tap] * self.fir[source][(self.fir_cursor + tap) % TAPS];
                }
                self.ring[source][self.cursor] = value;
            }
            self.cursor = (self.cursor + 1) % FFT;
            self.filled = (self.filled + 1).min(FFT);
            self.since_hop += 1;
            if self.filled == FFT && self.since_hop >= HOP {
                self.since_hop = 0;
                self.analyze(first_frame + offset as u64 + 1);
            }
        }
        self.snapshot
    }

    fn fft(&mut self) {
        let shift = usize::BITS - FFT.trailing_zeros();
        for n in 0..FFT {
            let reverse = n.reverse_bits() >> shift;
            if reverse > n {
                self.real.swap(n, reverse);
                self.imag.swap(n, reverse);
            }
        }
        let mut width = 2;
        while width <= FFT {
            let half = width / 2;
            let stride = FFT / width;
            for start in (0..FFT).step_by(width) {
                for j in 0..half {
                    let a = start + j;
                    let b = a + half;
                    let t = j * stride;
                    let re = self.real[b] * self.twiddle_re[t] - self.imag[b] * self.twiddle_im[t];
                    let im = self.real[b] * self.twiddle_im[t] + self.imag[b] * self.twiddle_re[t];
                    self.real[b] = self.real[a] - re;
                    self.imag[b] = self.imag[a] - im;
                    self.real[a] += re;
                    self.imag[a] += im;
                }
            }
            width *= 2;
        }
    }

    fn analyze(&mut self, end_frame: u64) {
        let mut chroma = [0.0; 12];
        let mut levels = [0.0; N];
        let mut register_sum = 0.0;
        let mut register_weight = 0.0;
        for (source, level) in levels.iter_mut().enumerate() {
            let mut energy = 0.0;
            for n in 0..FFT {
                let sample = self.ring[source][(self.cursor + n) % FFT];
                energy += sample * sample;
                self.real[n] = sample * self.window[n];
                self.imag[n] = 0.0;
            }
            *level = (energy / FFT as f32).sqrt();
            if *level < 0.001 || self.weights[source] == 0.0 {
                continue;
            }
            self.fft();
            // Sum octave-folded peak powers, not complex samples: opposite-phase
            // guitars cannot cancel one another in the harmonic representation.
            let mut pitches = [0.0_f32; 60];
            for bin in 1..FFT / 2 {
                let note = self.bin_note[bin];
                if note == u8::MAX {
                    continue;
                }
                let power = self.real[bin] * self.real[bin] + self.imag[bin] * self.imag[bin];
                pitches[note as usize] = pitches[note as usize].max(power);
            }
            let pitch_sum: f32 = pitches.iter().sum();
            if pitch_sum > 1e-8
                && pitches.iter().copied().fold(0.0_f32, f32::max) >= pitch_sum * 0.12
            {
                register_sum += pitches
                    .iter()
                    .enumerate()
                    .map(|(n, p)| n as f32 * p)
                    .sum::<f32>()
                    / pitch_sum
                    * self.weights[source];
                register_weight += self.weights[source];
            }
            let mut source_chroma = [0.0; 12];
            for (note, power) in pitches.iter().enumerate() {
                source_chroma[note % 12] += power;
            }
            let sum: f32 = source_chroma.iter().sum();
            if sum > 1e-8 {
                let weight = self.weights[source];
                for (combined, power) in chroma.iter_mut().zip(source_chroma) {
                    *combined += weight * power / sum;
                }
            }
        }
        self.snapshot.expression.register_direction = self
            .expression
            .register((register_weight > 0.0).then_some(register_sum / register_weight.max(1e-8)));
        normalize(&mut chroma);
        self.snapshot.source_frame = end_frame;
        self.snapshot.ready = true;
        self.snapshot.chroma = chroma;
        self.movement_age = self.movement_age.saturating_add(1);
        if self.movement_age >= 8 {
            self.snapshot.movement = None;
        }
        if chroma.iter().sum::<f32>() < 0.5 {
            self.quiet_hops = self.quiet_hops.saturating_add(1);
            self.snapshot.prediction = None;
            self.snapshot.chord = None;
            self.snapshot.key = None;
            self.snapshot.movement = None;
            self.snapshot.section = None;
            self.candidate = None;
            self.confirmations = 0;
            self.previous_chord = None;
            self.ambiguous_hops = 0;
            self.section_chroma = [0.0; 12];
            self.section_levels = [0.0; N];
            self.section_count = 0;
            self.section_tracker.candidate = None;
            self.key_history[self.key_cursor] = chroma;
            self.key_roots[self.key_cursor] = 0;
            self.key_cursor = (self.key_cursor + 1) % KEY_HISTORY;
            self.key_count = (self.key_count + 1).min(KEY_HISTORY);
            if self.quiet_hops >= 16 {
                self.clear_context();
            }
            return;
        }
        self.quiet_hops = 0;
        let chord = detect_chord(chroma);
        if same_chord(chord, self.candidate) && chord.is_some() {
            self.confirmations = self.confirmations.saturating_add(1);
        } else {
            self.candidate = chord;
            self.confirmations = 1;
        }
        // A changing/ambiguous frame is explicitly unknown, not stale certainty.
        self.snapshot.chord = chord.filter(|_| self.confirmations >= 2);
        self.key_history[self.key_cursor] = chroma;
        self.key_roots[self.key_cursor] = self.snapshot.chord.map_or(0, |c| 1 << c.root);
        self.key_cursor = (self.key_cursor + 1) % KEY_HISTORY;
        self.key_count = (self.key_count + 1).min(KEY_HISTORY);
        let previous_key = self.snapshot.key.map(|key| (key.tonic, key.mode));
        self.snapshot.key = self.estimate_key();
        if previous_key != self.snapshot.key.map(|key| (key.tonic, key.mode)) {
            self.snapshot.movement = None;
        }
        if self.snapshot.chord.is_none() {
            self.snapshot.prediction = None;
        }
        if chord.is_none() {
            self.ambiguous_hops = self.ambiguous_hops.saturating_add(1);
            self.snapshot.movement = None;
            // Mixed FFT windows at a real transition can be ambiguous briefly.
            // Preserve the last confirmed root for at most two such hops.
            if self.ambiguous_hops > 2 {
                self.previous_chord = None;
            }
        } else {
            self.ambiguous_hops = 0;
        }
        if let Some(chord) = self.snapshot.chord {
            if let Some(previous) = self.previous_chord
                && !same_chord(Some(previous), Some(chord))
            {
                let row = &mut self.transitions[chord_index(previous)];
                if row.iter().any(|n| *n >= 1024) {
                    row.iter_mut().for_each(|n| *n /= 2);
                }
                row[chord_index(chord)] = row[chord_index(chord)].saturating_add(1);
                self.snapshot.movement = Some(movement(previous, chord, self.snapshot.key));
                self.movement_age = 0;
            }
            self.previous_chord = Some(chord);
            let row = self.transitions[chord_index(chord)];
            let total: u32 = row.iter().map(|n| u32::from(*n)).sum();
            let (index, count) = row.iter().enumerate().max_by_key(|(_, n)| **n).unwrap();
            self.snapshot.prediction = (total >= 3 && u32::from(*count) * 4 >= total * 3)
                .then_some(ChordEstimate {
                    root: (index % 12) as u8,
                    quality: if index < 12 { Mode::Major } else { Mode::Minor },
                    confidence: *count as f32 / total.max(1) as f32,
                });
        }
        for (sum, value) in self.section_chroma.iter_mut().zip(chroma) {
            *sum += value;
        }
        for (sum, value) in self.section_levels.iter_mut().zip(levels) {
            *sum += value;
        }
        self.section_count += 1;
        if self.section_count >= SECTION_HOPS {
            let descriptor = Descriptor::new(self.section_chroma, self.section_levels);
            self.snapshot.section = Some(self.section_tracker.observe(descriptor));
            self.section_chroma = [0.0; 12];
            self.section_levels = [0.0; N];
            self.section_count = 0;
        }
    }

    fn clear_context(&mut self) {
        self.transitions = [[0; 24]; 24];
        self.key_history.fill([0.0; 12]);
        self.key_count = 0;
        self.key_cursor = 0;
        self.key_roots.fill(0);
        self.section_tracker = SectionTracker::default();
        self.section_chroma = [0.0; 12];
        self.section_levels = [0.0; N];
        self.section_count = 0;
    }

    fn estimate_key(&self) -> Option<KeyEstimate> {
        // A lone sustained triad is not evidence of the song's key.
        let roots = self
            .key_roots
            .iter()
            .fold(0_u16, |mask, roots| mask | roots);
        let active = self
            .key_history
            .iter()
            .filter(|c| c.iter().sum::<f32>() > 0.5)
            .count();
        if active < 48 || roots.count_ones() < 3 {
            return None;
        }
        let mut context = [0.0; 12];
        for frame in &self.key_history[..self.key_count] {
            for (sum, value) in context.iter_mut().zip(frame) {
                *sum += value;
            }
        }
        normalize(&mut context);
        // Explicit scale/tonic/dominant weighting; not a learned key detector.
        // Relative major/minor have the same scale, so a weak margin stays unknown.
        let profiles = [
            [1.0, 0.0, 0.45, 0.0, 0.7, 0.5, 0.0, 0.8, 0.0, 0.4, 0.0, 0.4],
            [1.0, 0.0, 0.4, 0.7, 0.0, 0.5, 0.0, 0.8, 0.4, 0.0, 0.45, 0.1],
        ];
        let mut best = (0.0_f32, 0_u8, Mode::Major);
        let mut second = 0.0_f32;
        for (mode_index, profile) in profiles.iter().enumerate() {
            for root in 0..12 {
                let score = cosine(
                    &context,
                    &std::array::from_fn(|pc| profile[(pc + 12 - root) % 12]),
                );
                if score > best.0 {
                    second = best.0;
                    best = (
                        score,
                        root as u8,
                        if mode_index == 0 {
                            Mode::Major
                        } else {
                            Mode::Minor
                        },
                    );
                } else {
                    second = second.max(score);
                }
            }
        }
        let margin = best.0 - second;
        if best.0 < 0.82 || margin < 0.025 {
            return None;
        }
        let (camelot_number, camelot_letter) = camelot(best.1, best.2);
        Some(KeyEstimate {
            tonic: best.1,
            mode: best.2,
            camelot_number,
            camelot_letter,
            confidence: (best.0 * (margin / 0.12).min(1.0)).clamp(0.0, 1.0),
        })
    }
}

fn normalize(values: &mut [f32; 12]) {
    let sum: f32 = values.iter().sum();
    if sum > 1e-8 {
        for value in values {
            *value /= sum;
        }
    }
}
fn cosine(a: &[f32; 12], b: &[f32; 12]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let aa: f32 = a.iter().map(|x| x * x).sum();
    let bb: f32 = b.iter().map(|x| x * x).sum();
    if aa * bb < 1e-12 {
        0.0
    } else {
        (dot / (aa * bb).sqrt()).clamp(0.0, 1.0)
    }
}
fn same_chord(a: Option<ChordEstimate>, b: Option<ChordEstimate>) -> bool {
    matches!((a,b), (Some(a),Some(b)) if a.root == b.root && a.quality == b.quality)
}
fn detect_chord(chroma: [f32; 12]) -> Option<ChordEstimate> {
    let norm = chroma.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < 1e-6 {
        return None;
    }
    let mut best = (0.0_f32, 0, Mode::Major, 0.0_f32);
    let mut second = 0.0_f32;
    for root in 0..12 {
        for (third, mode) in [(4, Mode::Major), (3, Mode::Minor)] {
            let tones = [
                chroma[root],
                chroma[(root + third) % 12],
                chroma[(root + 7) % 12],
            ];
            let score = tones.iter().sum::<f32>() / (3.0_f32.sqrt() * norm);
            let coverage = tones.into_iter().fold(f32::INFINITY, f32::min);
            if score > best.0 {
                second = best.0;
                best = (score, root, mode, coverage);
            } else {
                second = second.max(score);
            }
        }
    }
    let margin = best.0 - second;
    if best.0 < 0.82 || margin < 0.07 || best.3 < 0.075 {
        return None;
    }
    Some(ChordEstimate {
        root: best.1 as u8,
        quality: best.2,
        confidence: (best.0 * (margin / 0.3).min(1.0)).clamp(0.0, 1.0),
    })
}

/// Integer Camelot labels; valid tonic is C=0..B=11. Invalid input returns (0,'?').
pub fn camelot(tonic: u8, mode: Mode) -> (u8, char) {
    if tonic > 11 {
        return (0, '?');
    }
    let fifth = (tonic * 7) % 12;
    match mode {
        Mode::Major => ((fifth + 7) % 12 + 1, 'B'),
        Mode::Minor => ((fifth + 4) % 12 + 1, 'A'),
    }
}
fn movement(
    previous: ChordEstimate,
    current: ChordEstimate,
    key: Option<KeyEstimate>,
) -> HarmonicMovement {
    let delta = ((i16::from(current.root) - i16::from(previous.root)) * 7).rem_euclid(12);
    let steps = if delta > 6 { delta - 12 } else { delta } as i8;
    let kind = match steps {
        0 => MovementKind::Stationary,
        1 => MovementKind::FifthsForward,
        -1 => MovementKind::FifthsBackward,
        _ => MovementKind::Other,
    };
    let mut tension = 0.0;
    let mut resolution = 0.0;
    if let Some(key) = key {
        let relative = (current.root + 12 - key.tonic) % 12;
        let scale = match key.mode {
            Mode::Major => [0, 2, 4, 5, 7, 9, 11],
            Mode::Minor => [0, 2, 3, 5, 7, 8, 10],
        };
        tension = if relative == 0 {
            0.0
        } else if relative == 7 {
            0.6
        } else if scale.contains(&relative) {
            0.3
        } else {
            0.9
        };
        if previous.root == (key.tonic + 7) % 12
            && previous.quality == Mode::Major
            && current.root == key.tonic
            && current.quality == key.mode
        {
            resolution = previous
                .confidence
                .min(current.confidence)
                .min(key.confidence);
        }
    }
    HarmonicMovement {
        fifths_steps: steps,
        kind,
        tension,
        resolution,
    }
}

#[derive(Clone, Copy, Debug)]
struct Descriptor<const N: usize> {
    chroma: [f32; 12],
    balance: [f32; N],
    energy: f32,
}
impl<const N: usize> Default for Descriptor<N> {
    fn default() -> Self {
        Self {
            chroma: [0.0; 12],
            balance: [0.0; N],
            energy: 0.0,
        }
    }
}
impl<const N: usize> Descriptor<N> {
    fn new(mut chroma: [f32; 12], levels: [f32; N]) -> Self {
        normalize(&mut chroma);
        let sum: f32 = levels.iter().sum();
        Self {
            chroma,
            balance: levels.map(|x| if sum > 1e-6 { x / sum } else { 0.0 }),
            energy: sum / (SECTION_HOPS * N) as f32,
        }
    }
    fn distance(self, other: Self) -> f32 {
        let harmonic = 1.0 - cosine(&self.chroma, &other.chroma);
        let balance = self
            .balance
            .iter()
            .zip(other.balance)
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
            * 0.5;
        let energy = (self.energy - other.energy).abs() / self.energy.max(other.energy).max(0.001);
        (0.2 * harmonic + 0.35 * balance + 0.45 * energy).clamp(0.0, 1.0)
    }
    fn blend(self, other: Self) -> Self {
        Self {
            chroma: std::array::from_fn(|i| (self.chroma[i] + other.chroma[i]) * 0.5),
            balance: std::array::from_fn(|i| (self.balance[i] + other.balance[i]) * 0.5),
            energy: (self.energy + other.energy) * 0.5,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct SectionMemory<const N: usize> {
    descriptor: Descriptor<N>,
    id: u32,
    visits: u16,
}
#[derive(Clone, Debug)]
struct SectionTracker<const N: usize> {
    memory: [SectionMemory<N>; SECTION_MEMORY],
    used: usize,
    replace: usize,
    current: usize,
    next_id: u32,
    dwell: u16,
    candidate: Option<Descriptor<N>>,
}
impl<const N: usize> Default for SectionTracker<N> {
    fn default() -> Self {
        Self {
            memory: [SectionMemory::default(); SECTION_MEMORY],
            used: 0,
            replace: 0,
            current: 0,
            next_id: 1,
            dwell: 0,
            candidate: None,
        }
    }
}
impl<const N: usize> SectionTracker<N> {
    fn observe(&mut self, descriptor: Descriptor<N>) -> SectionEstimate {
        let mut recurrence = 0.0;
        let mut novelty = 0.0;
        if self.used == 0 {
            self.add(descriptor);
        } else {
            novelty = descriptor.distance(self.memory[self.current].descriptor);
            // Require sustained novelty (two 2.048 s blocks) and >=4 s dwell.
            // Harmony alone cannot reach this threshold: chord changes aren't sections.
            if novelty > 0.32 && self.dwell >= 2 {
                if self
                    .candidate
                    .is_some_and(|candidate| descriptor.distance(candidate) < 0.18)
                {
                    let mut matching = None;
                    let mut distance = 0.22;
                    for i in 0..self.used {
                        if i == self.current {
                            continue;
                        }
                        let candidate_distance = descriptor.distance(self.memory[i].descriptor);
                        if candidate_distance < distance {
                            distance = candidate_distance;
                            matching = Some(i);
                        }
                    }
                    if let Some(index) = matching {
                        self.current = index;
                        self.memory[index].visits = self.memory[index].visits.saturating_add(1);
                        recurrence = 1.0 - distance;
                    } else {
                        self.add(descriptor);
                    }
                    self.dwell = 0;
                    self.candidate = None;
                } else {
                    self.candidate = Some(descriptor);
                }
            } else {
                self.candidate = None;
                if novelty < 0.18 {
                    self.memory[self.current].descriptor =
                        self.memory[self.current].descriptor.blend(descriptor);
                }
            }
        }
        self.dwell = self.dwell.saturating_add(1);
        let current = self.memory[self.current];
        if current.visits > 1 {
            recurrence = recurrence.max(1.0 - descriptor.distance(current.descriptor));
        }
        // A recurring, contrasted arrangement can be a verse/chorus candidate;
        // no vocals, lyrics or formal song semantics are claimed by this heuristic.
        let mut label = SectionLabel::Unknown;
        let mut confidence = 0.0;
        if current.visits > 1 && recurrence > 0.78 {
            for other in &self.memory[..self.used] {
                if other.id == current.id || other.visits < 2 {
                    continue;
                }
                if current.descriptor.energy > other.descriptor.energy * 1.7 {
                    label = SectionLabel::ChorusCandidate;
                    confidence = 0.55 * recurrence;
                    break;
                }
                if other.descriptor.energy > current.descriptor.energy * 1.7 {
                    label = SectionLabel::VerseCandidate;
                    confidence = 0.55 * recurrence;
                }
            }
        }
        if current.visits == 1
            && self.dwell >= 2
            && self.memory[..self.used]
                .iter()
                .filter(|m| m.id != current.id && m.visits >= 2)
                .count()
                >= 2
        {
            label = SectionLabel::BridgeCandidate;
            confidence = 0.35;
        }
        SectionEstimate {
            id: current.id,
            novelty,
            recurrence,
            label,
            confidence,
        }
    }
    fn add(&mut self, descriptor: Descriptor<N>) {
        let index = if self.used < SECTION_MEMORY {
            let i = self.used;
            self.used += 1;
            i
        } else {
            let i = self.replace;
            self.replace = (self.replace + 1) % SECTION_MEMORY;
            i
        };
        self.memory[index] = SectionMemory {
            descriptor,
            id: self.next_id,
            visits: 1,
        };
        self.next_id = self.next_id.saturating_add(1);
        self.current = index;
    }
}

fn chord_index(chord: ChordEstimate) -> usize {
    chord.root as usize + if chord.quality == Mode::Minor { 12 } else { 0 }
}
