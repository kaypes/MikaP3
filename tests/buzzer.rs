//! Run on the host without linking the embedded HAL:
//! rustc --edition 2024 --test tests/buzzer.rs -o target/buzzer-tests
//! target/buzzer-tests

#[path = "../src/buzzer/playback.rs"]
mod playback;
mod buzzer {
    pub use crate::playback::Note;
}
#[path = "../src/songs/mod.rs"]
mod songs;

use playback::{Note, Output, Player, Tone};
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

#[derive(Clone, Default)]
struct FakeOutput(Rc<RefCell<Vec<Option<Tone>>>>);

impl Output for FakeOutput {
    fn set_tone(&mut self, tone: Option<Tone>) {
        self.0.borrow_mut().push(tone);
    }
}

impl FakeOutput {
    fn current(&self) -> Option<Tone> {
        self.0.borrow().last().copied().flatten()
    }

    fn writes(&self) -> usize {
        self.0.borrow().len()
    }
}

fn note(freq: u32, vol: u8, duration_ms: u64) -> Note {
    Note {
        freq,
        vol,
        duration_ms,
    }
}

fn player(
    tracks: [&[Note]; 2],
    now_us: u64,
    repeat: bool,
) -> (Player<'_, FakeOutput, FakeOutput>, FakeOutput, FakeOutput) {
    let a = FakeOutput::default();
    let b = FakeOutput::default();
    let player = Player::new(
        a.clone(),
        b.clone(),
        tracks,
        125_000_000,
        Some(now_us),
        repeat,
    )
    .unwrap();
    (player, a, b)
}

fn all_tracks() -> Vec<&'static [Note]> {
    vec![
        songs::always_with_me::TRACK_A,
        songs::always_with_me::TRACK_B,
        songs::fallen_down::TRACK_A,
        songs::fallen_down::TRACK_B,
        songs::game_of_thrones::TRACK_A,
        songs::game_of_thrones::TRACK_B,
        songs::hino_nacional::TRACK_A,
        songs::hino_nacional::TRACK_B,
        songs::his_theme::TRACK_A,
        songs::his_theme::TRACK_B,
        songs::le_festin::TRACK_A,
        songs::le_festin::TRACK_B,
        songs::love_like_you::TRACK_A,
        songs::love_like_you::TRACK_B,
        songs::married_life::TRACK_A,
        songs::married_life::TRACK_B,
        songs::minuet_in_g_major::TRACK_A,
        songs::minuet_in_g_major::TRACK_B,
        songs::new_horizons::TRACK_A,
        songs::new_horizons::TRACK_B,
        songs::tetris::TRACK_A,
        songs::tetris::TRACK_B,
    ]
}

#[test]
fn every_current_note_is_in_tune_at_both_clocks() {
    for clock in [125_000_000, 133_000_000] {
        for note in all_tracks()
            .into_iter()
            .flatten()
            .filter(|n| n.freq > 0 && n.vol > 0)
        {
            let tone = Tone::for_note(clock, *note, 127).unwrap();
            let actual = f64::from(clock) * 16.0
                / (f64::from(tone.divider_x16) * (f64::from(tone.top) + 1.0));
            let error = (actual / f64::from(note.freq) - 1.0).abs();
            assert!(error < 0.001, "clock={clock}, note={note:?}, error={error}");
            assert!((16..=4095).contains(&tone.divider_x16));
            assert!(u32::from(tone.compare) * 2 <= u32::from(tone.top) + 1);
        }
    }
}

#[test]
fn frequency_limits_do_not_wrap_or_divide_by_zero() {
    for clock in [125_000_000, 133_000_000] {
        for freq in [0, 1, 7, clock / 2 + 1, u32::MAX] {
            assert_eq!(Tone::for_note(clock, note(freq, 127, 1), 127), None);
        }
        for freq in [8, 15, 16, 440, 1_000_000, clock / 2] {
            assert!(Tone::for_note(clock, note(freq, 127, 1), 127).is_some());
        }
        let highest = Tone::for_note(clock, note(clock / 2, 127, 1), 127).unwrap();
        assert_eq!(
            (highest.top, highest.compare, highest.divider_x16),
            (1, 1, 16)
        );
    }
    assert!(Tone::for_note(0, note(440, 127, 1), 127).is_none());
    assert!(Tone::for_note(u32::MAX, note(u32::MAX, 127, 1), 127).is_none());
}

#[test]
fn period_includes_top_plus_one_and_uses_the_smallest_divider() {
    let tone = Tone::for_note(125_000_000, note(1000, 127, 1), 127).unwrap();
    assert_eq!((tone.divider_x16, tone.top), (31, 64515));
    assert!(125_000_000_u64 * 16 > 1000 * 30 * 65_536);
    let tone = Tone::for_note(125_000_000, note(62_500, 127, 1), 127).unwrap();
    assert_eq!((tone.divider_x16, tone.top), (16, 1999));
}

#[test]
fn volume_is_shared_across_tracks_and_clamped_to_midi() {
    let track_a = [note(440, 100, 100), note(440, 255, 0)];
    let track_b = [note(440, 50, 100), note(0, 127, 100)];
    let (mut p, a, b) = player([&track_a, &track_b], 0, false);
    p.update(0);
    let high = a.current().unwrap();
    let low = b.current().unwrap();
    assert_eq!(u32::from(high.compare), (u32::from(high.top) + 1) / 2);
    assert!((i32::from(high.compare) - i32::from(low.compare) * 2).abs() <= 1);
    assert_eq!(
        Tone::for_note(125_000_000, note(440, 255, 1), 127),
        Tone::for_note(125_000_000, note(440, 127, 1), 127)
    );
    for vol in 0..=255 {
        let tone = Tone::for_note(125_000_000, note(440, vol, 1), 127);
        if vol == 0 {
            assert!(tone.is_none());
        } else {
            let tone = tone.unwrap();
            assert!(tone.compare > 0);
            assert!(u32::from(tone.compare) * 2 <= u32::from(tone.top) + 1);
        }
    }
}

#[test]
fn silence_and_invalid_frequencies_keep_their_duration() {
    let track = [
        note(440, 0, 100),
        note(1, 127, 100),
        note(0, 127, 100),
        note(880, 127, 100),
    ];
    let (mut p, a, _) = player([&track, &[]], 0, false);
    for now in [0, 100_000, 200_000] {
        assert_eq!(p.update(now).next_wake_us, Some(now + 100_000));
        assert!(a.current().is_none());
    }
    p.update(300_000);
    assert!(a.current().is_some());
    assert!(p.update(400_000).finished);
    assert!(a.current().is_none());
}

#[test]
fn late_wakes_skip_expired_notes_without_moving_the_deadlines() {
    let a_notes = [
        note(440, 127, 100),
        note(660, 127, 100),
        note(880, 127, 100),
    ];
    let b_notes = [note(220, 127, 150), note(330, 127, 150)];
    let (mut p, a, b) = player([&a_notes, &b_notes], 1_000_000, false);
    assert_eq!(p.update(1_000_000).next_wake_us, Some(1_100_000));
    assert_eq!(p.update(1_225_000).next_wake_us, Some(1_300_000));
    assert_eq!(a.current(), Tone::for_note(125_000_000, a_notes[2], 127));
    assert_eq!(b.current(), Tone::for_note(125_000_000, b_notes[1], 127));
    assert!(p.update(1_310_000).finished);
}

#[test]
fn repeated_pauses_preserve_the_remaining_note_at_microsecond_precision() {
    let notes = [note(440, 127, 100), note(880, 127, 100)];
    let (mut p, a, b) = player([&notes, &notes], 0, false);
    p.update(0);
    p.set_paused(true, 40_123);
    assert!(a.current().is_none() && b.current().is_none());
    assert_eq!(p.update(500_000).next_wake_us, None);
    p.set_paused(true, 600_000);
    p.set_paused(false, 1_000_000);
    p.set_paused(false, 1_010_000);
    assert_eq!(p.update(1_010_000).next_wake_us, Some(1_059_877));
    assert_eq!(a.current(), Tone::for_note(125_000_000, notes[0], 127));
    p.set_paused(true, 1_020_000);
    p.set_paused(false, 2_000_000);
    assert_eq!(p.update(2_000_000).next_wake_us, Some(2_039_877));
    p.update(2_039_877);
    assert_eq!(a.current(), Tone::for_note(125_000_000, notes[1], 127));
    assert!(p.update(2_139_877).finished);
}

#[test]
fn pause_during_a_rest_does_not_start_the_next_note_early() {
    let notes = [note(0, 0, 100), note(440, 127, 100)];
    let (mut p, a, _) = player([&notes, &[]], 0, false);
    p.update(0);
    p.set_paused(true, 40_000);
    p.set_paused(false, 1_000_000);
    assert_eq!(p.update(1_000_000).next_wake_us, Some(1_060_000));
    assert!(a.current().is_none());
    p.update(1_060_000);
    assert!(a.current().is_some());
}

#[test]
fn tracks_finish_independently_and_zero_duration_notes_are_skipped() {
    let a_notes = [note(1000, 127, 0), note(440, 127, 100)];
    let b_notes = [note(220, 127, 200)];
    let (mut p, a, b) = player([&a_notes, &b_notes], 0, false);
    p.update(0);
    assert_eq!(a.current(), Tone::for_note(125_000_000, a_notes[1], 127));
    assert!(!p.update(100_000).finished);
    assert!(a.current().is_none());
    assert!(b.current().is_some());
    assert!(p.update(200_000).finished);
    assert!(a.current().is_none() && b.current().is_none());
}

#[test]
fn looping_uses_the_longer_track_and_catches_up_across_multiple_cycles() {
    let a_notes = [note(440, 127, 100)];
    let b_notes = [note(220, 127, 200)];
    let (mut p, a, b) = player([&a_notes, &b_notes], 0, true);
    p.update(100_000);
    assert!(a.current().is_none() && b.current().is_some());
    assert_eq!(p.update(200_000).next_wake_us, Some(300_000));
    assert!(a.current().is_some() && b.current().is_some());
    assert_eq!(p.update(1_050_000).next_wake_us, Some(1_100_000));
    assert!(a.current().is_some());
    assert_eq!(p.update(1_155_000).next_wake_us, Some(1_200_000));
    assert!(a.current().is_none() && b.current().is_some());
}

#[test]
fn empty_tracks_and_zero_duration_loops_finish_without_spinning() {
    let zero = [note(440, 127, 0)];
    for repeat in [false, true] {
        let (mut p, a, b) = player([&zero, &[]], 0, repeat);
        let result = p.update(0);
        assert!(result.finished);
        assert_eq!(result.next_wake_us, None);
        assert!(a.current().is_none() && b.current().is_none());
    }
    let rests = [note(0, 0, 100)];
    let (mut p, a, b) = player([&rests, &rests], 0, false);
    assert!(!p.update(0).finished);
    assert!(a.current().is_none() && b.current().is_none());
    assert!(p.update(100_000).finished);
}

#[test]
fn overflowing_durations_are_rejected_with_both_outputs_muted() {
    let a = FakeOutput::default();
    let b = FakeOutput::default();
    let notes = [note(440, 127, u64::MAX)];
    assert!(
        Player::new(
            a.clone(),
            b.clone(),
            [&notes, &[]],
            125_000_000,
            Some(0),
            false
        )
        .is_none()
    );
    let notes = [note(440, 127, u64::MAX / 1000); 2];
    assert!(
        Player::new(
            a.clone(),
            b.clone(),
            [&notes, &[]],
            125_000_000,
            Some(0),
            false
        )
        .is_none()
    );
    assert!(a.current().is_none() && b.current().is_none());
    assert_eq!(a.writes(), 2);
    assert_eq!(b.writes(), 2);
}

#[test]
fn state_updates_do_not_reconfigure_an_unchanged_note() {
    let notes = [note(440, 127, 100)];
    let (mut p, a, b) = player([&notes, &notes], 0, false);
    p.update(0);
    for now in [10_000, 20_000, 90_000] {
        p.set_paused(false, now);
        assert_eq!(p.update(now).next_wake_us, Some(100_000));
    }
    assert_eq!(a.writes(), 2);
    assert_eq!(b.writes(), 2);
}

#[test]
fn dropping_a_paused_or_playing_session_mutes_both_outputs() {
    let notes = [note(440, 127, 100)];
    for paused in [false, true] {
        let (mut p, a, b) = player([&notes, &notes], 0, false);
        p.update(0);
        p.set_paused(paused, 40_000);
        drop(p);
        assert!(a.current().is_none() && b.current().is_none());
    }
}

#[test]
fn cancelling_a_pending_future_drops_the_output_guard() {
    struct Noop;
    impl Wake for Noop {
        fn wake(self: Arc<Self>) {}
    }
    let notes = [note(440, 127, 100)];
    let (mut p, a, b) = player([&notes, &notes], 0, false);
    let mut future = Box::pin(async move {
        p.update(0);
        std::future::pending::<()>().await;
        drop(p);
    });
    let waker = Waker::from(Arc::new(Noop));
    assert_eq!(
        future.as_mut().poll(&mut Context::from_waker(&waker)),
        Poll::Pending
    );
    assert!(a.current().is_some() && b.current().is_some());
    drop(future);
    assert!(a.current().is_none() && b.current().is_none());
}

#[test]
fn preparing_a_session_does_not_consume_the_first_note() {
    let notes = [note(440, 127, 100)];
    let a = FakeOutput::default();
    let b = FakeOutput::default();
    let mut p = Player::new(a.clone(), b, [&notes, &[]], 125_000_000, None, false).unwrap();
    p.set_paused(true, 500_000);
    assert_eq!(p.update(1_000_000).next_wake_us, None);
    assert!(a.current().is_none());
    p.set_paused(false, 2_000_000);
    assert_eq!(p.update(2_000_000).next_wake_us, Some(2_100_000));
    assert!(a.current().is_some());
}

#[test]
fn invalid_notes_do_not_reduce_the_normalized_volume_of_audible_notes() {
    let notes = [note(1, 127, 100), note(440, 50, 100)];
    let (mut p, a, _) = player([&notes, &[]], 0, false);
    p.update(100_000);
    let tone = a.current().unwrap();
    assert_eq!(u32::from(tone.compare), (u32::from(tone.top) + 1) / 2);
}
