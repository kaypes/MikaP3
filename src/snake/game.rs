#[derive(Clone, Copy, PartialEq)]
pub struct SnakeGame {
    pub head: (u8, u8),
    pub fruit: (u8, u8),
    pub obstacles: [(u8, u8); 3],
    pub dir: u8,
    pub score: u32,
    pub speed_ms: u32,
    pub game_over: bool,
}

impl SnakeGame {
    pub fn new(mut seed: u64) -> Self {
        let head: (u8, u8) = (2, 2);
        let mut obstacles: [(u8, u8); 3] = [(0, 0); 3];

        for i in 0..3 {
            loop {
                let pos: (u8, u8) = (
                    (Self::rand(&mut seed) % 5) as u8,
                    (Self::rand(&mut seed) % 5) as u8,
                );

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
            let pos: (u8, u8) = (
                (Self::rand(seed) % 5) as u8,
                (Self::rand(seed) % 5) as u8,
            );

            if pos != self.head && !self.obstacles.contains(&pos) {
                self.fruit = pos;
                break;
            }
        }
    }

pub fn handle_input(&mut self, x_val: u16, y_val: u16) {
        if x_val > 3000 {
            self.dir = 0;
        } else if x_val < 1000 {
            self.dir = 2;
        }

        if y_val > 3000 {
            self.dir = 1;
        } else if y_val < 1000 {
            self.dir = 3;
        }
    }

pub fn step(&mut self, mut seed: u64) {
        if self.game_over {
            return;
        }

        let mut nx: u8 = self.head.0;
        let mut ny: u8 = self.head.1;
        
        match self.dir {
            0 => ny = if ny == 0 { 4 } else { ny - 1 },             
            1 => nx = if nx == 4 { 0 } else { nx + 1 }, 
            2 => ny = if ny == 4 { 0 } else { ny + 1 }, 
            3 => nx = if nx == 0 { 4 } else { nx - 1 }, 
            _ => {}
        }

        let new_head: (u8, u8) = (nx, ny);

        if self.obstacles.contains(&new_head) {
            self.game_over = true;
            return;
        }

        self.head = new_head;

        if self.head == self.fruit {
            self.score += 1;
            self.speed_ms = self.speed_ms.saturating_sub(15).max(150);
            self.spawn_fruit(&mut seed);
        }
    }
}