#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Pull};
use embassy_rp::pwm::{Config as PwmConfig, Pwm};
use embassy_rp::Peripherals;
use embassy_rp::bind_interrupts;
use embassy_rp::adc::{Adc, Channel as AdcChannel, Config as AdcConfig, InterruptHandler as AdcInterruptHandler};

use embassy_futures::join::join;
use embassy_futures::select::{select, Either};

use cortex_m_rt as _;
use panic_halt as _;
use embassy_rp as _;

mod buzzer;
mod songs;
mod state;
mod input;

use state::{AppState, STATE};

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => AdcInterruptHandler;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p: Peripherals = embassy_rp::init(Default::default());

    let btn_a = Input::new(p.PIN_5, Pull::Up);
    let btn_b = Input::new(p.PIN_6, Pull::Up);
    let adc = Adc::new(p.ADC, Irqs, AdcConfig::default());
    let joy_x = AdcChannel::new_pin(p.PIN_26, Pull::None);
    let joy_y = AdcChannel::new_pin(p.PIN_27, Pull::None);

    let mut pwm_a = Pwm::new_output_b(p.PWM_SLICE2, p.PIN_21, Default::default());
    let mut pwm_b = Pwm::new_output_a(p.PWM_SLICE5, p.PIN_10, Default::default());

    spawner.spawn(input::input_task(adc, joy_x, joy_y, btn_a, btn_b)).unwrap();

    let mut receiver = STATE.receiver().unwrap();

    loop {
        let current_state = receiver.get().await;

        match current_state {
            AppState::Menu { .. } => {
                let mut mute = PwmConfig::default();
                mute.compare_a = 0; mute.compare_b = 0;
                pwm_a.set_config(&mute); pwm_b.set_config(&mute);
                
                receiver.changed().await;
            }
            AppState::Playing { song_id, paused, art_id } => {
                if paused {
                    let mut mute = PwmConfig::default();
                    mute.compare_a = 0; mute.compare_b = 0;
                    pwm_a.set_config(&mute); pwm_b.set_config(&mute);
                    receiver.changed().await;
                } else {
                    let (track_a, track_b) = match song_id {
                        0 => (songs::married_life::TRACK_A, songs::married_life::TRACK_B),
                        1 => (songs::always_with_me::TRACK_A, songs::always_with_me::TRACK_B),
                        2 => (songs::fallen_down::TRACK_A, songs::fallen_down::TRACK_B),
                        3 => (songs::his_theme::TRACK_A, songs::his_theme::TRACK_B),
                        4 => (songs::love_like_you::TRACK_A, songs::love_like_you::TRACK_B),
                        5 => (songs::minuet_in_g_major::TRACK_A, songs::minuet_in_g_major::TRACK_B),
                        6 => (songs::new_horizons::TRACK_A, songs::new_horizons::TRACK_B),
                        _ => (songs::married_life::TRACK_A, songs::married_life::TRACK_B),
                    };

                    let play_future = join(
                        buzzer::play_track_a(&mut pwm_a, track_a),
                        buzzer::play_track_b(&mut pwm_b, track_b),
                    );
                    
                    let wait_future = receiver.changed();

                    match select(play_future, wait_future).await {
                        Either::First(_) => {
                            STATE.sender().send(AppState::Menu { song_id, art_id });
                        }
                        Either::Second(_) => {}
                    }
                }
            }
        }
    }
}