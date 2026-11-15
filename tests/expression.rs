use shr_lux::{
    expression::{Activity, Direction},
    musical::SourceMusicalAnalyzer,
};
use std::f32::consts::TAU;

fn block<const N: usize>(
    a: &mut SourceMusicalAnalyzer<N>,
    cursor: &mut u64,
    gains: [f32; N],
    note: f32,
) -> shr_lux::musical::MusicalSnapshot {
    let frames = std::array::from_fn::<_, 480, _>(|i| {
        let t = (*cursor + i as u64) as f32 / 48000.0;
        gains.map(|gain| gain * (TAU * 440.0 * 2.0_f32.powf((note - 69.0) / 12.0) * t).sin())
    });
    let s = a.process(&frames, *cursor);
    *cursor += 480;
    s
}
#[test]
fn seven_sources_include_vocals_and_do_not_confuse_missing_audio_with_pause() {
    let mut a = SourceMusicalAnalyzer::<7>::with_weights([0., 0.35, 1., 1., 0.7, 1., 0.7]).unwrap();
    let mut cursor = 0;
    for _ in 0..100 {
        block(&mut a, &mut cursor, [0., 0., 0., 0., 0., 0., 0.2], 60.);
    }
    assert_eq!(a.snapshot().expression.activity, Activity::Playing);
    for _ in 0..20 {
        block(&mut a, &mut cursor, [0.; 7], 60.);
    }
    assert_eq!(a.snapshot().expression.activity, Activity::BriefGap);
    for _ in 0..20 {
        block(&mut a, &mut cursor, [0.; 7], 60.);
    }
    assert_eq!(a.snapshot().expression.activity, Activity::Pause);
    let resumed = block(&mut a, &mut cursor, [0., 0., 0., 0., 0.2, 0., 0.], 60.);
    assert!(resumed.expression.resumed);
    assert_eq!(resumed.expression.activity, Activity::Playing);
    for _ in 0..310 {
        block(&mut a, &mut cursor, [0.; 7], 60.);
    }
    assert_eq!(a.snapshot().expression.activity, Activity::Silence);
    cursor += 480; // A lost block is unavailable, not silence.
    let s = block(&mut a, &mut cursor, [0.; 7], 60.);
    assert!(!s.ready);
    assert_eq!(s.expression.activity, Activity::Settling);
}
#[test]
fn register_and_energy_have_independent_directions() {
    let mut a = SourceMusicalAnalyzer::<1>::with_weights([1.]).unwrap();
    let mut cursor = 0;
    for _ in 0..100 {
        block(&mut a, &mut cursor, [0.2], 60.);
    }
    let mut up = false;
    for _ in 0..80 {
        up |= block(&mut a, &mut cursor, [0.2], 72.)
            .expression
            .register_direction
            == Direction::Up;
    }
    assert!(up);
    let mut down = false;
    for _ in 0..80 {
        down |= block(&mut a, &mut cursor, [0.2], 60.)
            .expression
            .register_direction
            == Direction::Down;
    }
    assert!(down);
    for _ in 0..100 {
        block(&mut a, &mut cursor, [0.1], 60.);
    }
    let s = block(&mut a, &mut cursor, [0.4], 60.);
    assert_ne!(s.expression.register_direction, Direction::Up);
    for _ in 0..30 {
        block(&mut a, &mut cursor, [0.4], 60.);
    }
    assert_eq!(a.snapshot().expression.energy_direction, Direction::Up);
}
#[test]
fn source_budget_is_explicit_and_not_six() {
    assert!(SourceMusicalAnalyzer::<0>::with_weights([]).is_err());
    assert!(SourceMusicalAnalyzer::<17>::with_weights([1.; 17]).is_err());
    assert!(SourceMusicalAnalyzer::<16>::with_weights([1.; 16]).is_ok());
    assert!(SourceMusicalAnalyzer::<6>::with_weights([f32::NAN; 6]).is_err());
}

#[test]
#[ignore = "bounded offline CPU measurement; run release explicitly, no devices"]
fn seven_feed_worker_cost() {
    let mut a = SourceMusicalAnalyzer::<7>::with_weights([0., 0.35, 1., 1., 1., 1., 1.]).unwrap();
    let frames = std::array::from_fn::<_, 480, _>(|i| {
        let t = i as f32 / 48000.;
        let tone = [200., 300., 400.]
            .iter()
            .map(|f| (TAU * f * t).sin())
            .sum::<f32>()
            * 0.15;
        [tone; 7]
    });
    let mut times = [std::time::Duration::ZERO; 3000];
    let start = std::time::Instant::now();
    for (block, duration) in times.iter_mut().enumerate() {
        let tick = std::time::Instant::now();
        std::hint::black_box(a.process(std::hint::black_box(&frames), block as u64 * 480));
        *duration = tick.elapsed();
    }
    let elapsed = start.elapsed();
    times.sort_unstable();
    println!(
        "7 feeds, 48 kHz, 30 simulated seconds: wall={elapsed:?}, mean single-core budget={:.2}%, 10ms block p99={:?}, max={:?}",
        elapsed.as_secs_f64() / 30. * 100.,
        times[2970],
        times[2999]
    );
}

#[test]
fn broadband_noise_has_no_register_direction_and_clipped_backing_feed_resets() {
    let mut a = SourceMusicalAnalyzer::<7>::with_weights([0., 0.35, 1., 1., 1., 1., 1.]).unwrap();
    let mut seed = 77_u32;
    for block in 0..100 {
        let frames = std::array::from_fn::<_, 480, _>(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let noise = (seed as f64 / u32::MAX as f64 * 2. - 1.) as f32 * 0.2;
            [0., 0., 0., 0., 0., 0., noise]
        });
        a.process(&frames, block * 480);
    }
    assert_eq!(
        a.snapshot().expression.register_direction,
        Direction::Unknown
    );
    let clipped = [[0., 0., 0., 0., 0., 0., 1.]; 480];
    let s = a.process(&clipped, 48000);
    assert!(!s.ready && s.prediction.is_none());
    assert_eq!(s.expression.activity, Activity::Settling);
}
