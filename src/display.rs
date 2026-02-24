use crate::state::{AppState, STATE};
use core::fmt::Write;
use embassy_rp::i2c::{Async, I2c};
use embassy_rp::peripherals::I2C1;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::iso_8859_1::{FONT_6X10, FONT_8X13};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text, TextStyleBuilder};
use ssd1306::{I2CDisplayInterface, Ssd1306, prelude::*};

const SONG_NAMES: [&str; 11] = [
    "Married Life",
    "Always with Me",
    "Fallen Down",
    "Game of Thrones",
    "Hino Nacional",
    "His Theme",
    "Le Festin",
    "Love Like You",
    "Minuet in G",
    "New Horizons",
    "Tetris",
];

const ART_NAMES: [&str; 11] = [
    "CORAÇÃO",
    "SORRISO",
    "COELHO",
    "SUPER MARIO",
    "FLOR",
    "FANSTASMA",
    "FOGUETE",
    "ESTRELA",
    "ESPADA",
    "NOTA MUSICAL",
    "FORMIGA",
];

#[embassy_executor::task]
pub async fn display_task(i2c: I2c<'static, I2C1, Async>) {
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    let style_song = MonoTextStyle::new(&FONT_8X13, BinaryColor::On);
    let style_art = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

    let text_style = TextStyleBuilder::new().alignment(Alignment::Center).build();

    let mut rx = STATE.receiver().unwrap();

    let mut last_score = u32::MAX;
    let mut last_state_type = u8::MAX;
    let mut last_game_over: bool = false;

    loop {
        let current_state: AppState = rx.get().await;
        let mut needs_flush: bool = false;

        match current_state {
            AppState::Menu { song_id, art_id } => {
                last_state_type = 0;
                needs_flush = true;

                display.clear(BinaryColor::Off).unwrap();
                Text::with_text_style("== MikaP3 ==", Point::new(64, 10), style_art, text_style)
                    .draw(&mut display).unwrap();
                Text::with_text_style(SONG_NAMES[song_id as usize], Point::new(64, 30), style_song, text_style)
                    .draw(&mut display).unwrap();

                let mut art_text = heapless::String::<32>::new();
                write!(&mut art_text, "Arte: {}", ART_NAMES[art_id as usize]).unwrap();
                Text::with_text_style(&art_text, Point::new(64, 45), style_art, text_style)
                    .draw(&mut display).unwrap();
                Text::with_text_style("B: Tocar", Point::new(64, 60), style_art, text_style)
                    .draw(&mut display).unwrap();
            }

            AppState::Playing { song_id, art_id, paused } => {
                last_state_type = 1;
                needs_flush = true;

                display.clear(BinaryColor::Off).unwrap();
                if paused {
                    Text::with_text_style("[ PAUSADO ]", Point::new(64, 10), style_art, text_style)
                        .draw(&mut display).unwrap();
                } else {
                    Text::with_text_style("TOCANDO...", Point::new(64, 10), style_art, text_style)
                        .draw(&mut display).unwrap();
                }

                Text::with_text_style(SONG_NAMES[song_id as usize], Point::new(64, 30), style_song, text_style)
                    .draw(&mut display).unwrap();

                let mut art_text = heapless::String::<32>::new();
                write!(&mut art_text, "Arte: {}", ART_NAMES[art_id as usize]).unwrap();
                Text::with_text_style(&art_text, Point::new(64, 45), style_art, text_style)
                    .draw(&mut display).unwrap();

                Text::with_text_style("A: Voltar   B: Pausar", Point::new(64, 60), style_art, text_style)
                    .draw(&mut display).unwrap();
            }

            AppState::Snake(game, _) => {
                if last_state_type != 2 || game.score != last_score || game.game_over != last_game_over {
                    last_state_type = 2;
                    last_score = game.score;
                    last_game_over = game.game_over;
                    needs_flush = true;

                    display.clear(BinaryColor::Off).unwrap();
                    let mut score_text = heapless::String::<32>::new();
                    write!(&mut score_text, "PONTOS: {}", game.score).unwrap();

                    Text::with_text_style(&score_text, Point::new(64, 35), style_song, text_style)
                        .draw(&mut display).unwrap();

                    if game.game_over {
                        Text::with_text_style("GAME OVER", Point::new(64, 55), style_art, text_style)
                            .draw(&mut display).unwrap();
                    }
                }
            }
        }

        if needs_flush {
            display.flush().unwrap();
        }
        
        rx.changed().await;
    }
}