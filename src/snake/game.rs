#[derive(Clone, Copy, PartialEq)]
pub struct SnakeGame {
    pub head: (i8, i8),
    pub fruit: (i8, i8),
    pub obstacles: [(i8, i8); 3],
    pub dir: u8,
    pub score: u32,
    pub speed_ms: u32,
    pub game_over: bool,
}

impl SnakeGame {
    pub fn new(mut seed: u64) -> Self {
        let head: (i8, i8) = (2, 2);
        let mut obstacles: [(i8, i8); 3] = [(0, 0); 3];

        for i in 0..3 {
            loop {
                let ox: i8 = (Self::rand(&mut seed) % 5) as i8;
                let oy: i8 = (Self::rand(&mut seed) % 5) as i8;
                let pos: (i8, i8) = (ox, oy);

                if pos != head && !obstacles[0..i].contains(&pos) {
                    obstacles[i] = pos;
                    break;
                }
            }
        }

        let mut game = Self {
            head,
            fruit: (0, 0),
            obstacles,
            dir: 1,
            score: 0,
            speed_ms: 500,
            game_over: false,
        };

        game.spawn_fruit(&mut seed);
        game
    }

    fn rand(seed: &mut u64) -> u64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        *seed
    }

    fn spawn_fruit(&mut self, seed: &mut u64) {
        loop {
            let fx: i8 = (Self::rand(seed) % 5) as i8;
            let fy: i8 = (Self::rand(seed) % 5) as i8;
            let pos: (i8, i8) = (fx, fy);

            if pos != self.head && !self.obstacles.contains(&pos) {
                self.fruit = pos;
                break;
            }
        }
    }

    pub fn handle_input(&mut self, x_val: u16, y_val: u16) {
        if x_val < 1000 && self.dir != 1 {
            self.dir = 3;
        } else if x_val > 3000 && self.dir != 3 {
            self.dir = 1;
        } else if y_val < 1000 && self.dir != 2 {
            self.dir = 0;
        } else if y_val > 3000 && self.dir != 0 {
            self.dir = 2;
        }
    }

    pub fn step(&mut self, mut seed: u64) {
        if self.game_over {
            return;
        }

        let mut nx: i8 = self.head.0;
        let mut ny: i8 = self.head.1;

        match self.dir {
            0 => ny -= 1,
            1 => nx += 1,
            2 => ny += 1,
            3 => nx -= 1,
            _ => {}
        }

        if nx < 0 {
            nx = 4;
        } else if nx > 4 {
            nx = 0;
        }

        if ny < 0 {
            ny = 4;
        } else if ny > 4 {
            ny = 0;
        }

        let new_head: (i8, i8) = (nx, ny);

        if self.obstacles.contains(&new_head) {
            self.game_over = true;
            return;
        }

        self.head = new_head;

        if self.head == self.fruit {
            self.score += 1;
            self.speed_ms = self.speed_ms.saturating_sub(15);
            self.spawn_fruit(&mut seed);
        }
    }
}
