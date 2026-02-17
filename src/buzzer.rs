use embassy_rp::pwm::{Config, Pwm};
use embassy_time::{Duration, Instant, Timer};

#[derive(Copy, Clone)]
pub struct Note {
    pub freq: u32,
    pub vol: u8,
    pub duration_ms: u64,
}

pub async fn play_track_a(pwm: &mut Pwm<'_>, track: &[Note]) {
    let volume_divisor: u32 = 8;

    for note in track {
        let end_time = Instant::now() + Duration::from_millis(note.duration_ms);

        if note.freq == 0 || note.vol == 0 {
            let mut mute = Config::default();
            mute.compare_b = 0;
            pwm.set_config(&mute);
        } else {
            let mut config = Config::default();
            config.divider = 125.into();
            let top = (1_000_000 / note.freq) as u16;
            config.top = top;
            
            let mut duty_cycle = (top as u32 * note.vol as u32) / (volume_divisor * 127);
            
            if duty_cycle == 0 { 
                duty_cycle = 1; 
            }
            
            config.compare_b = duty_cycle as u16; 
            pwm.set_config(&config);
        }

        Timer::at(end_time).await;
    }

    let mut mute = Config::default();
    mute.compare_b = 0;
    pwm.set_config(&mute);
}

pub async fn play_track_b(pwm: &mut Pwm<'_>, track: &[Note]) {
    let volume_divisor: u32 = 8;

    for note in track {
        let end_time = Instant::now() + Duration::from_millis(note.duration_ms);

        if note.freq == 0 || note.vol == 0 {
            let mut mute = Config::default();
            mute.compare_a = 0;
            pwm.set_config(&mute);
        } else {
            let mut config = Config::default();
            config.divider = 125.into();
            let top = (1_000_000 / note.freq) as u16;
            config.top = top;
            
            let mut duty_cycle = (top as u32 * note.vol as u32) / (volume_divisor * 127);
            
            if duty_cycle == 0 { 
                duty_cycle = 1; 
            }
            
            config.compare_a = duty_cycle as u16; 
            pwm.set_config(&config);
        }

        Timer::at(end_time).await;
    }

    let mut mute = Config::default();
    mute.compare_a = 0;
    pwm.set_config(&mute);
}