use embassy_rp::i2c::{Async, I2c};
use embassy_rp::peripherals::I2C1;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use crate::state::{AppState, STATE};

const SONG_NAMES: [&str; 7] = [
    "Married Life", "Always with Me", "Fallen Down", "His Theme", 
    "Love Like You", "Minuet in G Major", "New Horizons"
];

const ART_NAMES: [&str; 7] = [
    "1", "2", "3", "4", 
    "5", "6", "7"
];

#[embassy_executor::task]
pub async fn display_task(i2c: I2c<'static, I2C1, Async>) {
    let interface = I2CDisplayInterface::new(i2c);
    
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    
    display.init().unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    let mut rx = STATE.receiver().unwrap();

    loop {
        let current_state = rx.get().await;

        display.clear(BinaryColor::Off).unwrap();

        match current_state {
            AppState::Menu { song_id, art_id } => {
                Text::with_baseline("== MikaP3 ==", Point::new(10, 5), text_style, Baseline::Top)
                    .draw(&mut display).unwrap();

                let mut song_text = heapless::String::<32>::new();
                use core::fmt::Write;
                write!(&mut song_text, "> Musica: {}", SONG_NAMES[song_id as usize]).unwrap();
                Text::with_baseline(&song_text, Point::new(0, 25), text_style, Baseline::Top)
                    .draw(&mut display).unwrap();

                let mut art_text = heapless::String::<32>::new();
                write!(&mut art_text, "  Arte: {}", ART_NAMES[art_id as usize]).unwrap();
                Text::with_baseline(&art_text, Point::new(0, 40), text_style, Baseline::Top)
                    .draw(&mut display).unwrap();
                
                Text::with_baseline("A: Tocar", Point::new(40, 55), text_style, Baseline::Top)
                    .draw(&mut display).unwrap();
            }
            
            AppState::Playing { song_id, art_id: _, paused } => {
                let mut title_text = heapless::String::<32>::new();
                use core::fmt::Write;
                write!(&mut title_text, "Tocando: {}", SONG_NAMES[song_id as usize]).unwrap();
                
                Text::with_baseline(&title_text, Point::new(0, 15), text_style, Baseline::Top)
                    .draw(&mut display).unwrap();

                if paused {
                    Text::with_baseline("[ PAUSADO ]", Point::new(30, 35), text_style, Baseline::Top)
                        .draw(&mut display).unwrap();
                } else {
                    Text::with_baseline("||||||||||||||", Point::new(20, 35), text_style, Baseline::Top)
                        .draw(&mut display).unwrap();
                }

                Text::with_baseline("B: Voltar  A: Pausar", Point::new(0, 55), text_style, Baseline::Top)
                    .draw(&mut display).unwrap();
            }
        }

        display.flush().unwrap();

        rx.changed().await;
    }
}