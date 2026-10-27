#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::adc::{
    Adc, Channel as AdcChannel, Config as AdcConfig, InterruptHandler as AdcInterruptHandler,
};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Pull};
use embassy_rp::i2c::{Config as I2cConfig, I2c, InterruptHandler as I2cInterruptHandler};
use embassy_rp::pio::{InterruptHandler as PioInterruptHandler, Pio};
use embassy_rp::pwm::Pwm;
use embassy_rp::{Peripherals, peripherals};

use embassy_futures::select::select;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::Receiver;
use embassy_time::Timer;

use cortex_m_rt as _;
use embassy_rp as _;
use panic_halt as _;

mod arts;
mod buzzer;
mod display;
mod flappy;
mod input;
mod leds;
mod snake;
mod songs;
mod state;

use state::{AppState, STATE};

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => AdcInterruptHandler;
    I2C1_IRQ => I2cInterruptHandler<peripherals::I2C1>;
    PIO0_IRQ_0 => PioInterruptHandler<peripherals::PIO0>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p: Peripherals = embassy_rp::init(Default::default());

    let btn_a = Input::new(p.PIN_5, Pull::Up);
    let btn_b = Input::new(p.PIN_6, Pull::Up);
    let btn_joy = Input::new(p.PIN_22, Pull::Up);
    let adc = Adc::new(p.ADC, Irqs, AdcConfig::default());
    let joy_x = AdcChannel::new_pin(p.PIN_26, Pull::None);
    let joy_y = AdcChannel::new_pin(p.PIN_27, Pull::None);

    let mut pwm_a = Pwm::new_output_b(p.PWM_SLICE2, p.PIN_21, Default::default());
    let mut pwm_b = Pwm::new_output_a(p.PWM_SLICE5, p.PIN_10, Default::default());

    let sda = p.PIN_14;
    let scl = p.PIN_15;

    let mut i2c_config = I2cConfig::default();
    i2c_config.frequency = 400_000;
    let i2c = I2c::new_async(p.I2C1, scl, sda, Irqs, i2c_config);

    spawner
        .spawn(input::input_task(adc, joy_x, joy_y, btn_a, btn_b, btn_joy))
        .unwrap();
    spawner.spawn(display::display_task(i2c)).unwrap();

    let mut pio = Pio::new(p.PIO0, Irqs);
    let led_pin = pio.common.make_pio_pin(p.PIN_7);

    spawner
        .spawn(leds::leds_task(pio.common, pio.sm0, led_pin))
        .unwrap();

    let mut receiver = STATE.receiver().unwrap();

    loop {
        let current_state = receiver.get().await;
        let tracks = match current_state {
            AppState::Playing { song_id, .. } => Some(song_tracks(song_id)),
            AppState::Snake(_, true) => {
                Some((songs::hino_nacional::TRACK_A, songs::hino_nacional::TRACK_B))
            }
            AppState::FlappyBird(_, true) => Some((songs::tetris::TRACK_A, songs::tetris::TRACK_B)),
            _ => None,
        };

        if let Some((track_a, track_b)) = tracks {
            play_music(
                &mut pwm_a,
                &mut pwm_b,
                &mut receiver,
                current_state,
                track_a,
                track_b,
            )
            .await;
        } else {
            buzzer::mute(&mut pwm_a, &mut pwm_b);
            receiver.changed().await;
        }
    }
}

fn song_tracks(song_id: u8) -> (&'static [buzzer::Note], &'static [buzzer::Note]) {
    match song_id {
        0 => (songs::married_life::TRACK_A, songs::married_life::TRACK_B),
        1 => (
            songs::always_with_me::TRACK_A,
            songs::always_with_me::TRACK_B,
        ),
        2 => (songs::fallen_down::TRACK_A, songs::fallen_down::TRACK_B),
        3 => (
            songs::game_of_thrones::TRACK_A,
            songs::game_of_thrones::TRACK_B,
        ),
        4 => (songs::hino_nacional::TRACK_A, songs::hino_nacional::TRACK_B),
        5 => (songs::his_theme::TRACK_A, songs::his_theme::TRACK_B),
        6 => (songs::le_festin::TRACK_A, songs::le_festin::TRACK_B),
        7 => (songs::love_like_you::TRACK_A, songs::love_like_you::TRACK_B),
        8 => (
            songs::minuet_in_g_major::TRACK_A,
            songs::minuet_in_g_major::TRACK_B,
        ),
        9 => (songs::new_horizons::TRACK_A, songs::new_horizons::TRACK_B),
        10 => (songs::tetris::TRACK_A, songs::tetris::TRACK_B),
        _ => (songs::married_life::TRACK_A, songs::married_life::TRACK_B),
    }
}

async fn play_music<'d>(
    pwm_a: &mut Pwm<'d>,
    pwm_b: &mut Pwm<'d>,
    receiver: &mut Receiver<'_, CriticalSectionRawMutex, AppState, 4>,
    initial_state: AppState,
    track_a: &[buzzer::Note],
    track_b: &[buzzer::Note],
) {
    let repeat = !matches!(initial_state, AppState::Playing { .. });
    let Some(mut session) = buzzer::Session::new(pwm_a, pwm_b, track_a, track_b, repeat) else {
        // An unrepresentable track duration must not spin or leave sound on.
        if let AppState::Playing {
            song_id, art_id, ..
        } = receiver.get().await
        {
            STATE.sender().send(AppState::Menu { song_id, art_id });
        } else {
            receiver.changed().await;
        }
        return;
    };

    loop {
        // Read the latest state before completion too, so a simultaneous exit
        // wins over the end timer and the latest artwork survives completion.
        let current_state = receiver.get().await;
        let paused = match (initial_state, current_state) {
            (
                AppState::Playing {
                    song_id: original, ..
                },
                AppState::Playing {
                    song_id, paused, ..
                },
            ) if original == song_id => paused,
            (AppState::Snake(_, true), AppState::Snake(_, true))
            | (AppState::FlappyBird(_, true), AppState::FlappyBird(_, true)) => false,
            _ => return,
        };
        session.set_paused(paused);
        let (finished, deadline) = session.update();
        if finished {
            if let AppState::Playing {
                song_id, art_id, ..
            } = current_state
            {
                STATE.sender().send(AppState::Menu { song_id, art_id });
                return;
            }
            // Empty looping tracks wait for input instead of busy-looping.
            receiver.changed().await;
        } else if let Some(deadline) = deadline {
            // State changes take priority when both futures are ready.
            select(receiver.changed(), Timer::at(deadline)).await;
        } else {
            receiver.changed().await;
        }
    }
}
