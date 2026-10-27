#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlappyGame {
    pub bird_y: i32,
    pub pipe_x: i32,
    pub pipe_gap: i32,
    pub score: u32,
    pub game_over: bool,
    pub speed_ms: u32,
    jump_queued: bool,
    rng_seed: u32,
}

impl FlappyGame {
    pub fn new(seed: u64) -> Self {
        let mut game = Self {
            bird_y: 2,
            pipe_x: 4,
            pipe_gap: 2,
            score: 0,
            game_over: false,
            speed_ms: 400,
            jump_queued: false,
            rng_seed: seed as u32,
        };
        game.randomize_gap();
        game
    }

    fn randomize_gap(&mut self) {
        self.rng_seed = self.rng_seed.wrapping_mul(1103515245).wrapping_add(12345);
        self.pipe_gap = (self.rng_seed % 3) as i32 + 1;
    }

    pub fn jump(&mut self) {
        if !self.game_over {
            self.jump_queued = true;
        }
    }

    pub fn step(&mut self, _now_ticks: u64) {
        if self.game_over {
            return;
        }

        if self.jump_queued {
            self.bird_y -= 1;
            self.jump_queued = false;
        } else {
            self.bird_y += 1;
        }

        self.pipe_x -= 1;

        if self.bird_y < 0 || self.bird_y > 4 {
            self.game_over = true;
            return;
        }

        if self.pipe_x == 1 && self.bird_y != self.pipe_gap && self.bird_y != (self.pipe_gap - 1) {
            self.game_over = true;
            return;
        }

        if self.pipe_x < 0 {
            self.pipe_x = 4;
            self.score += 1;
            self.randomize_gap();

            if self.speed_ms > 150 {
                self.speed_ms -= 15;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jumps_are_queued_once_and_gravity_returns_on_the_next_step() {
        let mut game = FlappyGame::new(42);
        game.jump();
        game.jump();
        game.step(0);
        assert_eq!(game.bird_y, 1);
        game.step(1);
        assert_eq!(game.bird_y, 2);
    }

    #[test]
    fn scoring_resets_the_pipe_and_collisions_end_the_game() {
        let mut game = FlappyGame::new(42);
        game.pipe_x = 0;
        game.step(0);
        assert_eq!((game.score, game.pipe_x, game.speed_ms), (1, 4, 385));
        assert!((1..=3).contains(&game.pipe_gap));
        game.pipe_x = 2;
        game.pipe_gap = 1;
        game.step(1);
        assert!(game.game_over);
        let stopped = game;
        game.jump();
        game.step(2);
        assert_eq!(game, stopped);
    }
}
