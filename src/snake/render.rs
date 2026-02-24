use super::game::SnakeGame;

pub fn draw_frame(game: &SnakeGame) -> [(u8, u8, u8); 25] {
    let mut pixels: [(u8, u8, u8); 25] = [(0, 0, 0); 25];

    if game.game_over {
        for i in 0..25 {
            pixels[i] = (2, 0, 0);
        }
    } else {
        for obs in game.obstacles.iter() {
            let idx: usize = (obs.1 as usize * 5) + obs.0 as usize;
            pixels[idx] = (0, 0, 10);
        }

        let f_idx: usize = (game.fruit.1 as usize * 5) + game.fruit.0 as usize;
        pixels[f_idx] = (0, 10, 0);

        let h_idx: usize = (game.head.1 as usize * 5) + game.head.0 as usize;
        pixels[h_idx] = (10, 10, 0);
    }

    pixels
}