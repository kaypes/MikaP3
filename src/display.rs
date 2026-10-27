use core::fmt::Write;
use embassy_futures::select::select;
use embassy_rp::i2c::{Async, I2c};
use embassy_rp::peripherals::I2C1;
use embassy_time::{Instant, Timer};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::iso_8859_1::{FONT_6X10, FONT_8X13};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Alignment, Text, TextStyleBuilder};
use mikap3::visual::{DisplayView, OledCadence};
use ssd1306::{I2CDisplayInterface, Ssd1306, prelude::*};

#[embassy_executor::task]
pub async fn display_task(i2c: I2c<'static, I2C1, Async>) {
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();
    let style_song = MonoTextStyle::new(&FONT_8X13, BinaryColor::On);
    let style_art = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let text_style = TextStyleBuilder::new().alignment(Alignment::Center).build();
    let mut rx = crate::runtime::subscribe_state();
    let mut last_view = None;
    let mut cadence = OledCadence::default();
    loop {
        let view = DisplayView::from(rx.get().await);
        if last_view != Some(view) {
            let now = Instant::now().as_millis();
            if let Some(deadline) = cadence.wait_until(view, now) {
                select(Timer::at(Instant::from_millis(deadline)), rx.changed()).await;
                continue;
            }
            cadence.drawn(view, now);
            display.clear(BinaryColor::Off).unwrap();
            match view {
                DisplayView::Breakout(game) => {
                    mikap3::breakout::render::draw(&game, &mut display).unwrap();
                }
                DisplayView::Menu { song, art } | DisplayView::Playing { song, art, .. } => {
                    let playing = matches!(view, DisplayView::Playing { .. });
                    if !playing {
                        Text::with_text_style(
                            "== MikaP3 ==",
                            Point::new(64, 10),
                            style_art,
                            text_style,
                        )
                        .draw(&mut display)
                        .unwrap();
                    } else {
                        let title = if matches!(view, DisplayView::Playing { paused: true, .. }) {
                            "[ PAUSADO ]"
                        } else {
                            "TOCANDO..."
                        };
                        Text::with_text_style(title, Point::new(64, 10), style_art, text_style)
                            .draw(&mut display)
                            .unwrap();
                    }
                    Text::with_text_style(song, Point::new(64, 25), style_song, text_style)
                        .draw(&mut display)
                        .unwrap();
                    let mut art_text = heapless::String::<32>::new();
                    write!(&mut art_text, "Arte: {}", art).unwrap();
                    Text::with_text_style(&art_text, Point::new(64, 38), style_art, text_style)
                        .draw(&mut display)
                        .unwrap();
                }
                DisplayView::Score {
                    score, game_over, ..
                } => {
                    let mut score_text = heapless::String::<32>::new();
                    write!(&mut score_text, "PONTOS: {}", score).unwrap();
                    Text::with_text_style(&score_text, Point::new(64, 20), style_song, text_style)
                        .draw(&mut display)
                        .unwrap();
                    if game_over {
                        Text::with_text_style(
                            "GAME OVER",
                            Point::new(64, 35),
                            style_art,
                            text_style,
                        )
                        .draw(&mut display)
                        .unwrap();
                    }
                }
                DisplayView::EasterEgg => {
                    Text::with_text_style("<3", Point::new(64, 35), style_song, text_style)
                        .draw(&mut display)
                        .unwrap();
                }
                DisplayView::GamesMenu { game, with_music } => {
                    Text::with_text_style(
                        "== JOGOS ==",
                        Point::new(64, 10),
                        style_song,
                        text_style,
                    )
                    .draw(&mut display)
                    .unwrap();
                    Text::with_text_style(game.name(), Point::new(64, 25), style_song, text_style)
                        .draw(&mut display)
                        .unwrap();
                    let sound = if with_music {
                        "Som: [ ON ]"
                    } else {
                        "Som: [ OFF ]"
                    };
                    Text::with_text_style(sound, Point::new(64, 38), style_art, text_style)
                        .draw(&mut display)
                        .unwrap();
                }
            }
            for (row, hint) in view.control_hints().iter().enumerate() {
                Text::with_text_style(
                    hint,
                    Point::new(64, 50 + row as i32 * 12),
                    style_art,
                    text_style,
                )
                .draw(&mut display)
                .unwrap();
            }
            display.flush().unwrap();
            last_view = Some(view);
        }
        rx.changed().await;
    }
}
