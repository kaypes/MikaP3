#[derive(Clone, Copy, PartialEq)]
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

    pub fn input(&mut self, a: bool, b: bool, joy: bool) {
        if (a || b || joy) && !self.game_over {
            self.jump_queued = true;
        }
    }

    pub fn step(&mut self, _now_ticks: u64) {
        if self.game_over { return; }

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

        if self.pipe_x == 1 {
            if self.bird_y != self.pipe_gap && self.bird_y != (self.pipe_gap - 1) {
                self.game_over = true;
                return;
            }
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