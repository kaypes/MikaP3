//! OLED rendering on any monochrome DrawTarget, including host framebuffers.

use super::game::*;
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};

#[cfg(test)]
mod tests;

pub fn draw<D: DrawTarget<Color = BinaryColor>>(
    game: &BreakoutGame,
    target: &mut D,
) -> Result<(), D::Error> {
    let on = PrimitiveStyle::with_fill(BinaryColor::On);
    let style_text = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let mut buf = itoa::Buffer::new();
    let mut hud = heapless::String::<32>::new();
    hud.push_str("P:").unwrap();
    hud.push_str(buf.format(game.score.min(999_999))).unwrap();
    hud.push_str(" V:").unwrap();
    hud.push_str(buf.format(game.lives)).unwrap();
    hud.push_str(" F:").unwrap();
    hud.push_str(buf.format(game.level.min(9_999))).unwrap();
    Text::new(&hud, Point::new(1, 10), style_text).draw(target)?;
    Rectangle::new(Point::new(0, FIELD_TOP - 1), Size::new(128, 1))
        .into_styled(on)
        .draw(target)?;
    for x in [0, 127] {
        Rectangle::new(
            Point::new(x, FIELD_TOP),
            Size::new(1, (HEIGHT - FIELD_TOP) as u32),
        )
        .into_styled(on)
        .draw(target)?;
    }
    for index in 0..game.rows() * BRICK_COLUMNS {
        if game.has_brick(index) {
            let (x, y) = brick_position(index);
            Rectangle::new(
                Point::new(x, y),
                Size::new(BRICK_WIDTH as u32, BRICK_HEIGHT as u32),
            )
            .into_styled(on)
            .draw(target)?;
        }
    }
    Rectangle::new(
        Point::new(game.paddle_x(), PADDLE_Y),
        Size::new(PADDLE_WIDTH as u32, PADDLE_HEIGHT as u32),
    )
    .into_styled(on)
    .draw(target)?;
    let (x, y) = game.ball_position();
    Rectangle::new(
        Point::new(x, y),
        Size::new(BALL_SIZE as u32, BALL_SIZE.min(HEIGHT - y).max(0) as u32),
    )
    .into_styled(on)
    .draw(target)?;

    let lines = match game.status {
        Status::Serving => ["", "B:LANCAR A:VOLTAR"],
        Status::Playing => ["", ""],
        Status::Celebrating { .. } => ["FASE CONCLUIDA", "PROXIMA FASE..."],
        Status::GameOver => ["GAME OVER", "B:REINICIAR A:VOLTAR"],
    };
    for (row, line) in lines.iter().enumerate() {
        if !line.is_empty() {
            let y = 37 + row as i32 * 15;
            Rectangle::new(Point::new(1, y - 8), Size::new(126, 11))
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
                .draw(target)?;
            Text::with_alignment(
                line,
                Point::new(64, y),
                MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
                Alignment::Center,
            )
            .draw(target)?;
        }
    }
    Ok(())
}
