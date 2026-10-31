use super::*;
use core::convert::Infallible;

struct Framebuffer([u8; 1024]);

impl Framebuffer {
    fn lit(&self, x: usize, y: usize) -> bool {
        self.0[(y * 128 + x) / 8] & (1 << (x % 8)) != 0
    }
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        Size::new(128, 64)
    }
}

impl DrawTarget for Framebuffer {
    type Color = BinaryColor;
    type Error = Infallible;
    fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(
        &mut self,
        pixels: I,
    ) -> Result<(), Self::Error> {
        for Pixel(point, color) in pixels {
            assert!(
                (0..128).contains(&point.x) && (0..64).contains(&point.y),
                "out of bounds: {point:?}"
            );
            let index = point.y as usize * 128 + point.x as usize;
            let mask = 1 << (index % 8);
            if color == BinaryColor::On {
                self.0[index / 8] |= mask;
            } else {
                self.0[index / 8] &= !mask;
            }
        }
        Ok(())
    }
}

#[test]
fn draws_hud_bricks_bar_ball_and_arena_inside_a_128_by_64_frame() {
    let mut target = Framebuffer([0; 1024]);
    draw(&BrocosGame::new(), &mut target).unwrap();
    assert!((0..16).any(|y| (0..128).any(|x| target.lit(x, y))));
    assert!((0..128).all(|x| !target.lit(x, 15)));
    assert!((0..128).all(|x| target.lit(x, 16)));
    assert!(target.lit(0, 63) && target.lit(127, 63));
    assert!(target.lit(1, 18) && target.lit(14, 21));
    assert!(!target.lit(15, 18));
    assert!(target.lit(54, 60) && target.lit(73, 62));
    assert!(target.lit(63, 57) && target.lit(64, 58));
}

#[test]
fn all_status_messages_and_extreme_counters_fit_the_screen() {
    for status in [
        Status::Serving,
        Status::Playing,
        Status::Celebrating { started_ms: 0 },
        Status::GameOver,
    ] {
        let mut game = BrocosGame::new();
        game.score = u32::MAX;
        game.level = u32::MAX;
        game.status = status;
        draw(&game, &mut Framebuffer([0; 1024])).unwrap();
    }
}

#[test]
fn destroyed_brick_is_no_longer_drawn() {
    let mut game = BrocosGame::new();
    game.launch();
    for tick in 1..=30 {
        game.step(tick * 50);
        if game.score > 0 {
            break;
        }
    }
    assert!(game.score > 0);
    let missing = (0..24).find(|&index| !game.has_brick(index)).unwrap();
    let (x, y) = brick_position(missing);
    let mut target = Framebuffer([0; 1024]);
    draw(&game, &mut target).unwrap();
    for dy in 0..BRICK_HEIGHT {
        for dx in 0..BRICK_WIDTH {
            assert!(!target.lit((x + dx) as usize, (y + dy) as usize));
        }
    }
}

#[test]
fn ball_partially_below_the_bottom_edge_is_clipped_to_the_screen() {
    let game = crate::brocos::game::tests::about_to_miss(1);
    let mut target = Framebuffer([0; 1024]);
    draw(&game, &mut target).unwrap();
    assert!(target.lit(5, 63) && target.lit(6, 63));
}
