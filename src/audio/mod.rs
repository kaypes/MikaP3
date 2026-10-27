//! Hardware-independent PWM arithmetic and shared musical timeline.

#[cfg(test)]
mod tests;

/// The latest desired output, separate from visual and game state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioRequest {
    Silence,
    Play {
        session: u64,
        song_id: u8,
        repeat: bool,
        paused: bool,
    },
}

impl AudioRequest {
    pub fn session(self) -> Option<u64> {
        match self {
            Self::Silence => None,
            Self::Play { session, .. } => Some(session),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Note {
    pub freq: u32,
    /// MIDI intensity (0..=127), normalized jointly with the other track.
    pub vol: u8,
    pub duration_ms: u64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Tone {
    pub divider_x16: u16,
    pub top: u16,
    pub compare: u16,
}

impl Tone {
    pub fn for_note(clock_hz: u32, note: Note, peak: u8) -> Option<Self> {
        let volume = note.vol.min(127).min(peak);
        if note.freq == 0 || volume == 0 || peak == 0 || note.freq > clock_hz / 2 {
            return None;
        }
        let clock_x16 = u64::from(clock_hz) * 16;
        let frequency = u64::from(note.freq);
        let divider = clock_x16.div_ceil(frequency * 65_536).max(16);
        if divider > 4095 {
            return None;
        }
        let denominator = frequency * divider;
        let period = (clock_x16 + denominator / 2) / denominator;
        if !(2..=65_536).contains(&period) {
            return None;
        }
        let scale = u64::from(peak) * 2;
        let compare = ((period * u64::from(volume) + scale / 2) / scale).clamp(1, period / 2);
        Some(Self {
            divider_x16: u16::try_from(divider).ok()?,
            top: u16::try_from(period - 1).ok()?,
            compare: u16::try_from(compare).ok()?,
        })
    }
}

struct Track<'a> {
    notes: &'a [Note],
    index: usize,
    end_us: u64,
}

impl<'a> Track<'a> {
    fn new(notes: &'a [Note]) -> Self {
        Self {
            notes,
            index: 0,
            end_us: 0,
        }
    }

    fn at(&mut self, position_us: u64) -> Option<(Note, u64)> {
        while let Some(&note) = self.notes.get(self.index) {
            let end = self.end_us + note.duration_ms * 1000;
            if position_us < end {
                return Some((note, end));
            }
            self.end_us = end;
            self.index += 1;
        }
        None
    }

    fn reset(&mut self) {
        self.index = 0;
        self.end_us = 0;
    }
}

struct Playback<'a> {
    tracks: [Track<'a>; 2],
    duration_us: u64,
    position_us: u64,
    started_at_us: Option<u64>,
    repeat: bool,
    cycle: u64,
}

impl<'a> Playback<'a> {
    fn new(tracks: [&'a [Note]; 2], now_us: Option<u64>, repeat: bool) -> Option<Self> {
        let mut duration_us = 0;
        for track in tracks {
            let total = track.iter().try_fold(0_u64, |total, note| {
                total.checked_add(note.duration_ms.checked_mul(1000)?)
            })?;
            duration_us = duration_us.max(total);
        }
        Some(Self {
            tracks: tracks.map(Track::new),
            duration_us,
            position_us: 0,
            started_at_us: now_us,
            repeat,
            cycle: 0,
        })
    }

    fn position(&self, now_us: u64) -> u64 {
        self.position_us.saturating_add(
            self.started_at_us
                .map_or(0, |start| now_us.saturating_sub(start)),
        )
    }

    fn set_paused(&mut self, paused: bool, now_us: u64) {
        match (paused, self.started_at_us) {
            (true, Some(_)) => {
                self.position_us = self.position(now_us);
                self.started_at_us = None;
            }
            (false, None) => self.started_at_us = Some(now_us),
            _ => {}
        }
    }

    fn update(&mut self, now_us: u64) -> ([Option<Note>; 2], Update) {
        let elapsed = self.position(now_us);
        let finished = self.duration_us == 0 || (!self.repeat && elapsed >= self.duration_us);
        if finished || self.started_at_us.is_none() {
            return (
                [None; 2],
                Update {
                    finished,
                    next_wake_us: None,
                },
            );
        }
        let cycle = elapsed / self.duration_us;
        let position = elapsed % self.duration_us;
        if cycle != self.cycle {
            for track in &mut self.tracks {
                track.reset();
            }
            self.cycle = cycle;
        }
        let active = self.tracks.each_mut().map(|track| track.at(position));
        let next_end = active
            .iter()
            .flatten()
            .map(|(_, end)| *end)
            .min()
            .unwrap_or(self.duration_us);
        (
            active.map(|note| note.map(|(note, _)| note)),
            Update {
                finished: false,
                next_wake_us: Some(now_us.saturating_add(next_end - position)),
            },
        )
    }
}

pub trait Output {
    fn set_tone(&mut self, tone: Option<Tone>);
}

pub struct Update {
    pub finished: bool,
    pub next_wake_us: Option<u64>,
}

/// Output ownership makes completion, cancellation and early returns silent.
pub struct Player<'a, A: Output, B: Output> {
    a: A,
    b: B,
    playback: Playback<'a>,
    clock_hz: u32,
    peak: u8,
    last: [Option<Tone>; 2],
}

impl<'a, A: Output, B: Output> Player<'a, A, B> {
    pub fn new(
        mut a: A,
        mut b: B,
        tracks: [&'a [Note]; 2],
        clock_hz: u32,
        now_us: Option<u64>,
        repeat: bool,
    ) -> Option<Self> {
        a.set_tone(None);
        b.set_tone(None);
        let playback = Playback::new(tracks, now_us, repeat)?;
        let peak = tracks
            .iter()
            .flat_map(|track| track.iter())
            .filter(|note| note.duration_ms != 0 && Tone::for_note(clock_hz, **note, 127).is_some())
            .map(|note| note.vol.min(127))
            .max()
            .unwrap_or(0);
        Some(Self {
            a,
            b,
            playback,
            clock_hz,
            peak,
            last: [None; 2],
        })
    }

    pub fn set_paused(&mut self, paused: bool, now_us: u64) {
        self.playback.set_paused(paused, now_us);
        if paused {
            self.apply([None; 2]);
        }
    }

    pub fn update(&mut self, now_us: u64) -> Update {
        let (notes, update) = self.playback.update(now_us);
        self.apply(
            notes.map(|note| note.and_then(|note| Tone::for_note(self.clock_hz, note, self.peak))),
        );
        update
    }

    fn apply(&mut self, tones: [Option<Tone>; 2]) {
        if tones[0] != self.last[0] {
            self.a.set_tone(tones[0]);
        }
        if tones[1] != self.last[1] {
            self.b.set_tone(tones[1]);
        }
        self.last = tones;
    }
}

impl<A: Output, B: Output> Drop for Player<'_, A, B> {
    fn drop(&mut self) {
        self.a.set_tone(None);
        self.b.set_tone(None);
    }
}
