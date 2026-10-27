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

use cortex_m_rt as _;
use embassy_rp as _;
use panic_halt as _;

mod audio_task;
mod buzzer;
mod display;
mod input;
mod leds;
mod runtime;

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

    let pwm_a = Pwm::new_output_b(p.PWM_SLICE2, p.PIN_21, Default::default());
    let pwm_b = Pwm::new_output_a(p.PWM_SLICE5, p.PIN_10, Default::default());

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

    spawner.spawn(runtime::application_task()).unwrap();
    spawner.spawn(audio_task::audio_task(pwm_a, pwm_b)).unwrap();
}
