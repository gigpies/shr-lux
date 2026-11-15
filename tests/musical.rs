use shr_lux::{
    musical::{Mode, MovementKind, MusicalAnalyzer, MusicalSnapshot, SectionLabel, camelot},
    replay::AuxFrame,
};
use std::f32::consts::TAU;

fn feed(
    a: &mut MusicalAnalyzer,
    cursor: &mut u64,
    seconds: f32,
    notes: &[f32],
    gain: f32,
    opposite: bool,
) -> MusicalSnapshot {
    let mut snapshot = a.snapshot();
    for _ in 0..(seconds * 100.0) as usize {
        let frames: Vec<AuxFrame> = (0..480)
            .map(|n| {
                let t = (*cursor + n) as f32 / 48000.0;
                let tone = notes
                    .iter()
                    .map(|m| (TAU * 440.0 * 2.0_f32.powf((*m - 69.0) / 12.0) * t).sin())
                    .sum::<f32>()
                    * gain
                    / notes.len().max(1) as f32;
                [0.0, 0.0, tone, if opposite { -tone } else { 0.0 }]
            })
            .collect();
        snapshot = a.process(&frames, *cursor);
        *cursor += 480;
    }
    snapshot
}
#[test]
fn clean_triads_survive_source_phase_cancellation_and_distinguish_quality() {
    for (notes, root, mode) in [
        ([60.0, 64.0, 67.0], 0, Mode::Major),
        ([57.0, 60.0, 64.0], 9, Mode::Minor),
        ([62.0, 66.0, 69.0], 2, Mode::Major),
    ] {
        let mut a = MusicalAnalyzer::new();
        let mut cursor = 0;
        let s = feed(&mut a, &mut cursor, 1.0, &notes, 0.6, true);
        let chord = s.chord.expect("clean triad");
        assert_eq!((chord.root, chord.quality), (root, mode));
        assert!(s.ready && s.source_frame <= cursor && cursor - s.source_frame < 6144);
        assert!(s.key.is_none(), "single chord cannot establish song key");
    }
}
#[test]
fn power_chords_silence_invalid_pcm_and_gaps_are_unknown() {
    let mut a = MusicalAnalyzer::new();
    let mut cursor = 0;
    assert!(
        feed(&mut a, &mut cursor, 1.0, &[60.0, 67.0], 0.6, false)
            .chord
            .is_none()
    );
    feed(&mut a, &mut cursor, 1.0, &[60.0, 64.0, 67.0], 0.6, false);
    let s = feed(&mut a, &mut cursor, 0.5, &[], 0.0, false);
    assert!(s.ready && s.chord.is_none() && s.movement.is_none() && s.section.is_none());
    let bad = [[0.0, 0.0, 1.1, 0.0]; 480];
    assert!(!a.process(&bad, cursor).ready);
    let bad = [[0.0, 0.0, f32::NAN, 0.0]; 480];
    assert!(!a.process(&bad, cursor).ready);
    let s = a.process(&[[0.0; 4]; 480], cursor + 9000);
    assert!(!s.ready && s.source_frame == 0);
}
#[test]
fn fifth_movement_expires_and_does_not_bridge_silence() {
    let mut a = MusicalAnalyzer::new();
    let mut cursor = 0;
    feed(&mut a, &mut cursor, 1.0, &[60.0, 64.0, 67.0], 0.6, false);
    let s = feed(&mut a, &mut cursor, 0.65, &[67.0, 71.0, 74.0], 0.6, false);
    assert_eq!(
        s.movement.expect("confirmed fifth transition").kind,
        MovementKind::FifthsForward
    );
    assert!(
        feed(&mut a, &mut cursor, 1.2, &[67.0, 71.0, 74.0], 0.6, false)
            .movement
            .is_none()
    );
    feed(&mut a, &mut cursor, 0.5, &[], 0.0, false);
    assert!(
        feed(&mut a, &mut cursor, 0.8, &[60.0, 64.0, 67.0], 0.6, false)
            .movement
            .is_none()
    );
}
#[test]
fn camelot_matches_reference_pairs_and_invalid_tonic() {
    assert_eq!(camelot(0, Mode::Major), (8, 'B'));
    assert_eq!(camelot(9, Mode::Minor), (8, 'A'));
    assert_eq!(camelot(7, Mode::Major), (9, 'B'));
    assert_eq!(camelot(4, Mode::Minor), (9, 'A'));
    assert_eq!(camelot(12, Mode::Major), (0, '?'));
}
#[test]
fn repeated_contrasted_arrangements_get_local_identity_candidates() {
    let mut a = MusicalAnalyzer::new();
    let mut cursor = 0;
    let mut ids = Vec::new();
    let mut labels = Vec::new();
    for gain in [0.12, 0.65, 0.12, 0.65, 0.12] {
        let s = feed(&mut a, &mut cursor, 9.0, &[60.0, 64.0, 67.0], gain, false);
        let section = s.section.expect("arrangement descriptor");
        assert_eq!(shr_lux::scenes::section_identity(s), Some(section.id));
        ids.push(section.id);
        labels.push(section.label);
    }
    assert_eq!(ids[0], ids[2]);
    assert_eq!(ids[2], ids[4]);
    assert_eq!(ids[1], ids[3]);
    assert_ne!(ids[0], ids[1]);
    assert_eq!(labels[3], SectionLabel::ChorusCandidate);
    assert_eq!(labels[4], SectionLabel::VerseCandidate);
    a.reset();
    assert!(!a.snapshot().ready && a.snapshot().section.is_none());
}
#[test]
fn tonic_weighted_progression_supports_key_then_rolling_diversity_expires() {
    let mut a = MusicalAnalyzer::new();
    let mut cursor = 0;
    let mut key = None;
    for _ in 0..2 {
        for (notes, seconds) in [
            ([60.0, 64.0, 67.0], 3.0),
            ([65.0, 69.0, 72.0], 1.0),
            ([67.0, 71.0, 74.0], 1.0),
            ([60.0, 64.0, 67.0], 2.0),
        ] {
            key = feed(&mut a, &mut cursor, seconds, &notes, 0.6, false).key;
        }
    }
    let key = key.expect("tonic weighted I IV V I progression");
    assert_eq!(
        (key.tonic, key.mode, key.camelot_number, key.camelot_letter),
        (0, Mode::Major, 8, 'B')
    );
    assert!(
        feed(&mut a, &mut cursor, 13.0, &[60.0, 64.0, 67.0], 0.6, false)
            .key
            .is_none()
    );
}
#[test]
fn broadband_noise_and_clipped_ambiguous_input_do_not_fabricate_harmony() {
    let mut a = MusicalAnalyzer::new();
    let mut seed = 1_u32;
    for block in 0..100 {
        let frames: Vec<AuxFrame> = (0..480)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let x = (seed as f64 / u32::MAX as f64 * 2.0 - 1.0) as f32;
                [0.0, 0.0, x, 0.0]
            })
            .collect();
        let s = a.process(&frames, block * 480);
        assert!(s.chord.is_none() && s.key.is_none());
    }
    let s = a.process(&[[0.0, 0.0, 1.0, 0.0]; 480], 48000);
    assert!(s.chroma.iter().all(|x| x.is_finite()));
}
#[test]
fn harmonic_rich_guitars_with_opposite_phase_and_noisy_bass_remain_tonal() {
    let mut a = MusicalAnalyzer::new();
    let mut seed = 7_u32;
    for block in 0..100 {
        let frames: Vec<AuxFrame> = (0..480)
            .map(|n| {
                let t = (block * 480 + n) as f32 / 48000.0;
                let tone = [60.0, 64.0, 67.0]
                    .iter()
                    .map(|m| {
                        let phase = TAU * 440.0 * 2.0_f32.powf((*m - 69.0) / 12.0) * t;
                        phase.sin() + 0.2 * (phase * 2.0).sin() + 0.1 * (phase * 3.0).sin()
                    })
                    .sum::<f32>()
                    * 0.15;
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let noise = (seed as f64 / u32::MAX as f64 * 2.0 - 1.0) as f32 * 0.1;
                [0.0, noise, tone, -tone]
            })
            .collect();
        a.process(&frames, (block * 480) as u64);
    }
    let chord = a.snapshot().chord.expect("moderate harmonics/noisy bass");
    assert_eq!((chord.root, chord.quality), (0, Mode::Major));
}
#[test]
fn tonic_weighted_minor_with_major_dominant_supports_minor_key() {
    let mut a = MusicalAnalyzer::new();
    let mut cursor = 0;
    let mut key = None;
    for _ in 0..2 {
        for (notes, seconds) in [
            ([57.0, 60.0, 64.0], 3.0),
            ([62.0, 65.0, 69.0], 1.0),
            ([64.0, 68.0, 71.0], 1.0),
            ([57.0, 60.0, 64.0], 2.0),
        ] {
            key = feed(&mut a, &mut cursor, seconds, &notes, 0.6, false).key;
        }
    }
    let key = key.expect("tonic weighted minor progression with major dominant");
    assert_eq!(
        (key.tonic, key.mode, key.camelot_number, key.camelot_letter),
        (9, Mode::Minor, 8, 'A')
    );
}
#[test]
fn relative_mode_ambiguity_does_not_publish_confident_key() {
    let mut a = MusicalAnalyzer::new();
    let mut cursor = 0;
    for notes in [[60.0, 64.0, 67.0], [57.0, 60.0, 64.0], [65.0, 69.0, 72.0]] {
        feed(&mut a, &mut cursor, 0.6, &notes, 0.6, false);
    }
    // Uniform diatonic collection offers no independently supported tonic.
    let s = feed(
        &mut a,
        &mut cursor,
        10.0,
        &[60.0, 62.0, 64.0, 65.0, 67.0, 69.0, 71.0],
        0.6,
        false,
    );
    assert!(
        s.key.is_none(),
        "relative mode/tonic ambiguity: {:?}",
        s.key
    );
}

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
struct CountAllocator;
thread_local! { static ALLOCATION_COUNT: Cell<Option<usize>> = const {Cell::new(None)}; }
unsafe impl GlobalAlloc for CountAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATION_COUNT.with(|count| {
            if let Some(n) = count.get() {
                count.set(Some(n + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCATION_COUNT.with(|count| {
            if let Some(n) = count.get() {
                count.set(Some(n + 1));
            }
        });
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountAllocator = CountAllocator;
#[test]
fn processing_and_reset_allocate_nothing_after_construction() {
    let mut a = MusicalAnalyzer::new();
    let frames = std::array::from_fn::<AuxFrame, 480, _>(|n| {
        let t = n as f32 / 48000.0;
        let x = [200.0, 300.0, 400.0]
            .iter()
            .map(|f| (TAU * f * t).sin())
            .sum::<f32>()
            * 0.2;
        [0.0, x, x, -x]
    });
    let mut seven =
        shr_lux::musical::SourceMusicalAnalyzer::<7>::with_weights([0., 0.35, 1., 1., 1., 1., 1.])
            .unwrap();
    let seven_frames = frames.map(|f| [f[0], f[1], f[2], f[3], f[2], f[2], f[2]]);
    ALLOCATION_COUNT.with(|count| count.set(Some(0)));
    for block in 0..1500 {
        a.process(&frames, block * 480);
        seven.process(&seven_frames, block * 480);
    }
    a.reset();
    seven.reset();
    a.process(&frames, 720000);
    let count = ALLOCATION_COUNT.with(|count| count.replace(None).unwrap());
    assert_eq!(count, 0);
}
#[test]
fn overlapping_transition_with_all_tonal_feeds_has_bounded_ambiguity_grace() {
    let mut a = MusicalAnalyzer::new();
    let mut cursor = 0;
    let mut movement = None;
    let mut seed = 9_u32;
    for (notes, seconds, noise) in [
        ([60.0, 64.0, 67.0], 1.07, false),
        ([67.0, 71.0, 74.0], 0.65, false),
        ([0.0, 0.0, 0.0], 0.8, true),
        ([60.0, 64.0, 67.0], 0.8, false),
    ] {
        for _ in 0..(seconds * 100.0) as usize {
            let frames: Vec<AuxFrame> = (0..480)
                .map(|n| {
                    let t = (cursor + n) as f32 / 48000.0;
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let x = if noise {
                        (seed as f64 / u32::MAX as f64 * 2.0 - 1.0) as f32 * 0.5
                    } else {
                        notes
                            .iter()
                            .map(|m| (TAU * 440.0 * 2.0_f32.powf((*m - 69.0) / 12.0) * t).sin())
                            .sum::<f32>()
                            * 0.2
                    };
                    [0.0, x, x, -x]
                })
                .collect();
            let s = a.process(&frames, cursor);
            cursor += 480;
            if notes[0] == 67.0 {
                movement = movement.or(s.movement);
            }
            if cursor > 130000 {
                assert!(s.movement.is_none(), "long ambiguity must break continuity");
            }
        }
    }
    assert_eq!(
        movement.expect("real overlapping FFT transition").kind,
        MovementKind::FifthsForward
    );
}
#[test]
fn adc_clipping_erases_key_history_and_clean_audio_rewarms() {
    let mut a = MusicalAnalyzer::new();
    let mut cursor = 0;
    for _ in 0..2 {
        for (notes, seconds) in [
            ([60.0, 64.0, 67.0], 3.0),
            ([65.0, 69.0, 72.0], 1.0),
            ([67.0, 71.0, 74.0], 1.0),
            ([60.0, 64.0, 67.0], 2.0),
        ] {
            feed(&mut a, &mut cursor, seconds, &notes, 0.6, false);
        }
    }
    assert!(a.snapshot().key.is_some());
    let mut clipped = [[0.0; 4]; 480];
    for frame in &mut clipped[..3] {
        frame[0] = 0.999;
    }
    let s = a.process(&clipped, cursor);
    cursor += 480;
    assert_eq!(
        s,
        MusicalSnapshot::default(),
        "kick clipping resets all-source reliability too"
    );
    let s = feed(&mut a, &mut cursor, 0.2, &[60.0, 64.0, 67.0], 0.6, false);
    assert!(!s.ready, "new full spectral window required");
    let s = feed(&mut a, &mut cursor, 1.0, &[60.0, 64.0, 67.0], 0.6, false);
    assert!(
        s.ready && s.chord.is_some() && s.key.is_none(),
        "old key context erased"
    );
    // Two isolated saturated frames are tolerated by the base reliability gate.
    let mut allowed = [[0.0; 4]; 480];
    allowed[0][2] = 1.0;
    allowed[1][2] = -1.0;
    assert!(a.process(&allowed, cursor).ready);
    cursor += 480;
    let high = [[0.0, 0.998, 0.998, -0.998]; 480];
    assert!(
        a.process(&high, cursor).ready,
        "subthreshold signal accepted"
    );
}

#[test]
fn repeated_transitions_predict_only_with_observed_support_and_clear_on_gap() {
    let mut a = MusicalAnalyzer::new();
    let mut cursor = 0;
    let c = [60., 64., 67.];
    let g = [67., 71., 74.];
    feed(&mut a, &mut cursor, 0.8, &c, 0.4, false);
    assert!(a.snapshot().prediction.is_none());
    for _ in 0..4 {
        feed(&mut a, &mut cursor, 0.8, &g, 0.4, false);
        feed(&mut a, &mut cursor, 0.8, &c, 0.4, false);
    }
    let next = a.snapshot().prediction.expect("repeated C to G");
    assert_eq!(next.root, 7);
    assert!(next.confidence >= 0.75);
    cursor += 480;
    feed(&mut a, &mut cursor, 0.8, &c, 0.4, false);
    assert!(a.snapshot().prediction.is_none());
}

#[test]
fn novel_third_arrangement_after_recurrence_is_only_a_bridge_candidate() {
    let mut a = MusicalAnalyzer::new();
    let mut cursor = 0;
    for _ in 0..3 {
        feed(&mut a, &mut cursor, 9., &[60., 64., 67.], 0.12, false);
        feed(&mut a, &mut cursor, 9., &[60., 64., 67.], 0.6, true);
    }
    let s = feed(&mut a, &mut cursor, 10., &[62., 65., 69.], 0.3, false);
    let section = s.section.unwrap();
    assert_eq!(section.label, SectionLabel::BridgeCandidate);
    assert!(section.confidence <= 0.35);
}
