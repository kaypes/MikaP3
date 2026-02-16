#![no_std]

use embassy_rp::pwm::{Config, Pwm};
use embassy_time::{Duration, Timer};

#[derive(Copy, Clone)]
pub struct Note {
    pub freq_a: u32,
    pub freq_b: u32,
    pub duration_ms: u64,
}

pub async fn play_melody(pwm_a: &mut Pwm<'_>, pwm_b: &mut Pwm<'_>, melody: &[Note]) {
    let volume_divisor = 32;

    for note in melody {
        let mut config_a = Config::default();
        config_a.divider = 125.into();

        let mut config_b = Config::default();
        config_b.divider = 125.into();

        if note.freq_a == 0 && note.freq_b == 0 {
            config_a.compare_b = 0;
            config_b.compare_a = 0;
            pwm_a.set_config(&config_a);
            pwm_b.set_config(&config_b);
            Timer::after(Duration::from_millis(note.duration_ms)).await;
            continue;
        }

        if note.freq_a > 0 {
            let top_a = (1_000_000 / note.freq_a) as u16;
            config_a.top = top_a;
            config_a.compare_b = top_a / volume_divisor;
        } else {
            config_a.top = 1000;
            config_a.compare_b = 0;
        }

        // Configura a Nota B
        if note.freq_b > 0 {
            let top_b = (1_000_000 / note.freq_b) as u16;
            config_b.top = top_b;
            config_b.compare_a = top_b / volume_divisor;
        } else {
            config_b.top = 1000;
            config_b.compare_a = 0;
        }

        pwm_a.set_config(&config_a);
        pwm_b.set_config(&config_b);

        Timer::after(Duration::from_millis(note.duration_ms)).await;
    }

    let mut mute = Config::default();
    mute.compare_a = 0;
    mute.compare_b = 0;
    pwm_a.set_config(&mute);
    pwm_b.set_config(&mute);
}