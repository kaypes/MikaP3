#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnakeGame {
    pub head: (u8, u8),
    pub fruit: Option<(u8, u8)>,
    pub obstacles: [(u8, u8); 3],
    pub dir: Direction,
    pub score: u32,
    pub speed_ms: u32,
    pub game_over: bool,
    pub fruit_timer: u8,
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
            fruit: None,
            obstacles,
            dir: Direction::Idle,
            score: 0,
            speed_ms: 500,
            game_over: false,
            fruit_timer: 0,
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
            let pos: (u8, u8) = ((Self::rand(seed) % 5) as u8, (Self::rand(seed) % 5) as u8);

            if pos != self.head && !self.obstacles.contains(&pos) {
                self.fruit = Some(pos);
                break;
            }
        }
    }

    pub fn input(&mut self, direction: Direction) {
        self.dir = direction;
    }

    pub fn step(&mut self, mut seed: u64) {
        if self.game_over || self.dir == Direction::Idle {
            return;
        }

        let mut nx: u8 = self.head.0;
        let mut ny: u8 = self.head.1;

        match self.dir {
            Direction::Up => ny = if ny == 0 { 4 } else { ny - 1 },
            Direction::Right => nx = if nx == 4 { 0 } else { nx + 1 },
            Direction::Down => ny = if ny == 4 { 0 } else { ny + 1 },
            Direction::Left => nx = if nx == 0 { 4 } else { nx - 1 },
            Direction::Idle => {}
        }

        let new_head: (u8, u8) = (nx, ny);

        if self.obstacles.contains(&new_head) {
            self.game_over = true;
            return;
        }

        self.head = new_head;

        if self.fruit_timer > 0 {
            self.fruit_timer -= 1;

            if self.fruit_timer == 0 {
                self.spawn_fruit(&mut seed);
            }
        } else if Some(self.head) == self.fruit {
            self.score += 1;
            self.speed_ms = self.speed_ms.saturating_sub(15).max(150);
            self.fruit_timer = 1;
            self.fruit = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_direction_wraps_at_the_board_edge() {
        for (dir, expected) in [
            (Direction::Up, (0, 4)),
            (Direction::Right, (1, 0)),
            (Direction::Down, (0, 1)),
            (Direction::Left, (4, 0)),
        ] {
            let mut game = SnakeGame::new(42);
            game.head = (0, 0);
            game.obstacles = [(2, 2), (3, 2), (4, 2)];
            game.fruit = None;
            game.input(dir);
            game.step(123);
            assert_eq!(game.head, expected);
            assert!(!game.game_over);
        }
    }

    #[test]
    fn collision_stops_motion_and_eating_fruit_keeps_the_one_step_respawn() {
        let mut game = SnakeGame::new(42);
        game.obstacles = [(3, 2), (4, 2), (4, 4)];
        game.input(Direction::Right);
        game.step(123);
        assert!(game.game_over);
        assert_eq!(game.head, (2, 2));
        game.game_over = false;
        game.input(Direction::Left);
        game.fruit = Some((1, 2));
        game.step(124);
        assert_eq!((game.head, game.score, game.speed_ms), ((1, 2), 1, 485));
        assert_eq!(game.fruit, None);
        game.step(125);
        let fruit = game.fruit.unwrap();
        assert_ne!(fruit, game.head);
        assert!(!game.obstacles.contains(&fruit));
    }
}
