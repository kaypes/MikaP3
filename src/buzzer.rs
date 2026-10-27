mod playback;

use embassy_rp::clocks::clk_sys_freq;
use embassy_rp::pwm::{Config, Pwm};
use embassy_time::Instant;

pub use playback::Note;
use playback::{Output, Player, Tone};

struct PwmChannel<'a, 'd> {
    pwm: &'a mut Pwm<'d>,
    channel_b: bool,
}

impl Output for PwmChannel<'_, '_> {
    fn set_tone(&mut self, tone: Option<Tone>) {
        let mut config = Config::default();
        if let Some(tone) = tone {
            config.divider = fixed::types::U12F4::from_bits(tone.divider_x16);
            config.top = tone.top;
            if self.channel_b {
                config.compare_b = tone.compare;
            } else {
                config.compare_a = tone.compare;
            }
        }

        // TOP and compare are buffered while running. Stop before configuring
        // so a smaller TOP or a mute cannot leave the previous note sounding.
        let mut stopped = config.clone();
        stopped.enable = false;
        self.pwm.set_config(&stopped);
        self.pwm.set_counter(0);
        config.enable = tone.is_some();
        self.pwm.set_config(&config);
    }
}

/// Owns both output borrows for the lifetime of a song; dropping it mutes both.
pub struct Session<'a, 'd> {
    player: Player<'a, PwmChannel<'a, 'd>, PwmChannel<'a, 'd>>,
}

impl<'a, 'd> Session<'a, 'd> {
    pub fn new(
        pwm_a: &'a mut Pwm<'d>,
        pwm_b: &'a mut Pwm<'d>,
        track_a: &'a [Note],
        track_b: &'a [Note],
        repeat: bool,
    ) -> Option<Self> {
        Some(Self {
            player: Player::new(
                PwmChannel {
                    pwm: pwm_a,
                    channel_b: true,
                },
                PwmChannel {
                    pwm: pwm_b,
                    channel_b: false,
                },
                [track_a, track_b],
                clk_sys_freq(),
                // Begin at position zero after validation and normalization;
                // the caller's first set_paused(false) starts the clock.
                None,
                repeat,
            )?,
        })
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.player.set_paused(paused, Instant::now().as_micros());
    }

    /// Apply both notes and return completion plus the next absolute deadline.
    pub fn update(&mut self) -> (bool, Option<Instant>) {
        let update = self.player.update(Instant::now().as_micros());
        (
            update.finished,
            update.next_wake_us.map(Instant::from_micros),
        )
    }
}

pub fn mute(pwm_a: &mut Pwm<'_>, pwm_b: &mut Pwm<'_>) {
    PwmChannel {
        pwm: pwm_a,
        channel_b: true,
    }
    .set_tone(None);
    PwmChannel {
        pwm: pwm_b,
        channel_b: false,
    }
    .set_tone(None);
}
