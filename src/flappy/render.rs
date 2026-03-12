use super::game::FlappyGame;

pub fn draw_frame(game: &FlappyGame) -> [(u8, u8, u8); 25] {
    if game.game_over {
        return [(10, 0, 0); 25];
    }

    let mut pixels: [(u8, u8, u8); 25] = [(0, 0, 0); 25];

    let color_pipe: (u8, u8, u8) = (0, 5, 0);
    let color_bird: (u8, u8, u8) = (10, 5, 0);

    if game.pipe_x >= 0 && game.pipe_x < 5 {
        for y in 0..5 {
            if y != game.pipe_gap && y != (game.pipe_gap - 1) {
                let idx = (y * 5 + game.pipe_x) as usize;
                pixels[idx] = color_pipe;
            }
        }
    }

    if game.bird_y >= 0 && game.bird_y < 5 {
        let bird_idx = (game.bird_y * 5 + 1) as usize;
        pixels[bird_idx] = color_bird;
    }

    pixels
}
