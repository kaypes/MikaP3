//! Fixed-point Breakout physics. Coordinates describe the ball's top-left corner.

#[cfg(test)]
pub(crate) mod tests;

pub const FP: i32 = 256;
pub const WIDTH: i32 = 128;
pub const HEIGHT: i32 = 64;
pub const FIELD_TOP: i32 = 11;
pub const BALL_SIZE: i32 = 2;
pub const PADDLE_WIDTH: i32 = 20;
pub const PADDLE_Y: i32 = 60;
pub const PADDLE_HEIGHT: i32 = 3;
pub const BRICK_COLUMNS: usize = 8;
pub const BRICK_WIDTH: i32 = 14;
pub const BRICK_HEIGHT: i32 = 4;
pub const BRICK_TOP: i32 = 13;
pub const CELEBRATION_MS: u64 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Serving,
    Playing,
    Celebrating { started_ms: u64 },
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddleDirection {
    Left,
    Still,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Collision {
    Wall,
    Paddle,
    Brick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakoutGame {
    pub score: u32,
    pub lives: u8,
    pub level: u32,
    pub status: Status,
    bricks: u64,
    paddle_x: i32,
    ball_x: i32,
    ball_y: i32,
    vx: i32,
    vy: i32,
    direction: PaddleDirection,
}

impl Default for BreakoutGame {
    fn default() -> Self {
        Self::new()
    }
}

impl BreakoutGame {
    pub fn new() -> Self {
        let mut game = Self {
            score: 0,
            lives: 3,
            level: 1,
            status: Status::Serving,
            bricks: 0,
            paddle_x: (WIDTH - PADDLE_WIDTH) / 2,
            ball_x: 0,
            ball_y: 0,
            vx: 0,
            vy: 0,
            direction: PaddleDirection::Still,
        };
        game.reset_bricks();
        game.attach_ball();
        game
    }

    pub fn rows(&self) -> usize {
        3 + ((self.level - 1) % 3) as usize
    }
    pub fn has_brick(&self, index: usize) -> bool {
        index < self.rows() * BRICK_COLUMNS && self.bricks & (1 << index) != 0
    }
    pub fn paddle_x(&self) -> i32 {
        self.paddle_x
    }
    pub fn ball_position(&self) -> (i32, i32) {
        (self.ball_x / FP, self.ball_y / FP)
    }
    pub fn speed_fp(&self) -> i32 {
        2 * FP + (self.level.saturating_sub(1).min(8) as i32) * (FP / 4)
    }
    pub fn set_direction(&mut self, direction: PaddleDirection) {
        self.direction = direction;
    }

    pub fn launch(&mut self) {
        if self.status == Status::Serving {
            self.vx = self.speed_fp() / 2;
            self.vy = -self.speed_fp();
            self.status = Status::Playing;
        }
    }

    pub fn step(&mut self, now_ms: u64) -> Option<Collision> {
        if self.status == Status::GameOver {
            return None;
        }
        let movement = match self.direction {
            PaddleDirection::Left => -4,
            PaddleDirection::Still => 0,
            PaddleDirection::Right => 4,
        };
        self.paddle_x = (self.paddle_x + movement).clamp(1, WIDTH - 1 - PADDLE_WIDTH);
        if let Status::Celebrating { started_ms } = self.status {
            if now_ms.saturating_sub(started_ms) >= CELEBRATION_MS {
                self.level = self.level.saturating_add(1);
                self.reset_bricks();
                self.status = Status::Serving;
                self.attach_ball();
            }
            return None;
        }
        if self.status == Status::Serving {
            self.attach_ball();
            return None;
        }

        let steps = ((self.vx.abs().max(self.vy.abs()) + FP - 1) / FP).max(1);
        let mut collision = None;
        for part in 0..steps {
            let dx = self.vx * (part + 1) / steps - self.vx * part / steps;
            let mut x = self.ball_x + dx;
            let right = (WIDTH - 1 - BALL_SIZE) * FP;
            if x < FP || x > right {
                self.vx = -self.vx;
                x = x.clamp(FP, right);
                collision = collision.max(Some(Collision::Wall));
            }
            if let Some(index) = self.brick_at(x, self.ball_y) {
                self.destroy(index, now_ms);
                self.vx = -self.vx;
                collision = Some(Collision::Brick);
            } else {
                self.ball_x = x;
            }
            if matches!(self.status, Status::Celebrating { .. }) {
                break;
            }

            let dy = self.vy * (part + 1) / steps - self.vy * part / steps;
            let mut y = self.ball_y + dy;
            if y < FIELD_TOP * FP {
                y = FIELD_TOP * FP;
                self.vy = -self.vy;
                collision = collision.max(Some(Collision::Wall));
            }
            if let Some(index) = self.brick_at(self.ball_x, y) {
                self.destroy(index, now_ms);
                self.vy = -self.vy;
                collision = Some(Collision::Brick);
            } else if self.vy > 0
                && self.ball_y + BALL_SIZE * FP <= PADDLE_Y * FP
                && y + BALL_SIZE * FP >= PADDLE_Y * FP
                && self.ball_x + BALL_SIZE * FP > self.paddle_x * FP
                && self.ball_x < (self.paddle_x + PADDLE_WIDTH) * FP
            {
                self.ball_y = (PADDLE_Y - BALL_SIZE) * FP;
                self.rebound();
                collision = collision.max(Some(Collision::Paddle));
            } else if y >= HEIGHT * FP {
                self.lives -= 1;
                self.status = if self.lives == 0 {
                    Status::GameOver
                } else {
                    Status::Serving
                };
                self.attach_ball();
                break;
            } else {
                self.ball_y = y;
            }
            if matches!(self.status, Status::Celebrating { .. }) {
                break;
            }
        }
        collision
    }

    fn attach_ball(&mut self) {
        self.ball_x = (self.paddle_x + (PADDLE_WIDTH - BALL_SIZE) / 2) * FP;
        self.ball_y = (PADDLE_Y - BALL_SIZE - 1) * FP;
        self.vx = 0;
        self.vy = 0;
    }

    fn reset_bricks(&mut self) {
        self.bricks = (1_u64 << (self.rows() * BRICK_COLUMNS)) - 1;
    }

    fn brick_at(&self, x: i32, y: i32) -> Option<usize> {
        (0..self.rows() * BRICK_COLUMNS).find(|&index| {
            let (bx, by) = brick_position(index);
            self.has_brick(index)
                && x < (bx + BRICK_WIDTH) * FP
                && x + BALL_SIZE * FP > bx * FP
                && y < (by + BRICK_HEIGHT) * FP
                && y + BALL_SIZE * FP > by * FP
        })
    }

    fn destroy(&mut self, index: usize, now_ms: u64) {
        self.bricks &= !(1_u64 << index);
        self.score = self.score.saturating_add(10);
        if self.bricks == 0 {
            self.status = Status::Celebrating { started_ms: now_ms };
        }
    }

    fn rebound(&mut self) {
        let speed = self.speed_fp();
        let offset = self.ball_x + BALL_SIZE * FP / 2 - (self.paddle_x + PADDLE_WIDTH / 2) * FP;
        let horizontal = (offset * speed / (PADDLE_WIDTH / 2 * FP)).clamp(-speed, speed);
        self.vx = if horizontal.abs() < speed / 4 {
            if horizontal < 0 || (horizontal == 0 && self.vx < 0) {
                -speed / 4
            } else {
                speed / 4
            }
        } else {
            horizontal
        };
        self.vy = -speed;
    }
}

pub fn brick_position(index: usize) -> (i32, i32) {
    (
        1 + (index % BRICK_COLUMNS) as i32 * (BRICK_WIDTH + 2),
        BRICK_TOP + (index / BRICK_COLUMNS) as i32 * (BRICK_HEIGHT + 2),
    )
}
