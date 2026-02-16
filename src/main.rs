#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::pwm::Pwm;
use cortex_m_rt as _;
use panic_halt as _;
use embassy_rp as _;

mod buzzer;
mod songs;

use buzzer::play_melody;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut led_red = Output::new(p.PIN_13, Level::Low);
    let mut pwm_buzzer_a = Pwm::new_output_b(p.PWM_SLICE2, p.PIN_21, Default::default());
    let mut pwm_buzzer_b = Pwm::new_output_a(p.PWM_SLICE5, p.PIN_10, Default::default());

    loop {
        led_red.set_high();

        play_melody(&mut pwm_buzzer_a, &mut pwm_buzzer_b, songs::surtada_dada_boladao::GENERATED_SONG).await;

        led_red.set_low();
        embassy_time::Timer::after_secs(2).await;
    }
}