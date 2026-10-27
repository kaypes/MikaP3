use super::*;

pub(crate) fn about_to_hit_brick(last: bool) -> BreakoutGame {
    let mut game = BreakoutGame::new();
    game.status = Status::Playing;
    game.bricks = if last { 1 << 16 } else { (1 << 16) | (1 << 17) };
    game.ball_x = 3 * FP;
    game.ball_y = 30 * FP;
    game.vx = 0;
    game.vy = -4 * FP;
    game
}

pub(crate) fn about_to_miss(lives: u8) -> BreakoutGame {
    let mut game = BreakoutGame::new();
    game.status = Status::Playing;
    game.lives = lives;
    game.ball_x = 5 * FP;
    game.ball_y = 63 * FP;
    game.vx = 0;
    game.vy = 4 * FP;
    game
}

#[test]
fn starts_with_three_lives_twenty_four_bricks_and_a_ball_attached_to_the_bar() {
    let game = BreakoutGame::new();
    assert_eq!((game.lives, game.level, game.score), (3, 1, 0));
    assert_eq!(game.status, Status::Serving);
    assert_eq!(game.bricks.count_ones(), 24);
    assert_eq!(game.ball_position(), (63, 57));
}

#[test]
fn paddle_moves_four_pixels_stops_at_center_and_clamps_to_both_edges() {
    let mut game = BreakoutGame::new();
    game.set_direction(PaddleDirection::Left);
    game.step(50);
    assert_eq!((game.paddle_x(), game.ball_position().0), (50, 59));
    game.set_direction(PaddleDirection::Still);
    game.step(100);
    assert_eq!(game.paddle_x(), 50);
    game.set_direction(PaddleDirection::Left);
    for time in 0..100 {
        game.step(time);
    }
    assert_eq!(game.paddle_x(), 1);
    game.set_direction(PaddleDirection::Right);
    for time in 0..100 {
        game.step(time);
    }
    assert_eq!(game.paddle_x(), 107);
}

#[test]
fn launch_only_works_when_serving() {
    let mut game = BreakoutGame::new();
    game.launch();
    assert_eq!(game.status, Status::Playing);
    game.step(50);
    let moving = game;
    game.launch();
    assert_eq!(game, moving);
    assert_eq!(game.ball_position(), (64, 55));
}

#[test]
fn side_walls_and_top_corner_bounce_without_leaving_the_arena() {
    for (x, vx) in [(FP, -4 * FP), (125 * FP, 4 * FP)] {
        let mut game = BreakoutGame::new();
        game.status = Status::Playing;
        game.ball_x = x;
        game.ball_y = 45 * FP;
        game.vx = vx;
        game.vy = 0;
        assert_eq!(game.step(50), Some(Collision::Wall));
        assert!(game.vx.signum() != vx.signum());
        assert!((FP..=125 * FP).contains(&game.ball_x));
    }
    let mut game = BreakoutGame::new();
    game.status = Status::Playing;
    game.bricks = 1 << 39;
    game.ball_x = FP;
    game.ball_y = FIELD_TOP * FP;
    game.vx = -4 * FP;
    game.vy = -4 * FP;
    assert_eq!(game.step(50), Some(Collision::Wall));
    assert!(game.vx > 0 && game.vy > 0);
}

#[test]
fn paddle_rebound_depends_on_impact_point_and_keeps_vertical_motion() {
    for (offset, sign) in [(0, -1), (18, 1)] {
        let mut game = BreakoutGame::new();
        game.status = Status::Playing;
        game.ball_x = (game.paddle_x + offset) * FP;
        game.ball_y = 57 * FP;
        game.vx = 0;
        game.vy = 4 * FP;
        assert_eq!(game.step(50), Some(Collision::Paddle));
        assert_eq!(game.vx.signum(), sign);
        assert!(game.vy < 0 && game.ball_y <= 58 * FP);
    }
}

#[test]
fn simultaneous_collision_feedback_prioritizes_brick_then_paddle_then_wall() {
    let mut game = BreakoutGame::new();
    game.status = Status::Playing;
    game.paddle_x = 1;
    game.ball_x = FP;
    game.ball_y = 58 * FP;
    game.vx = -FP;
    game.vy = 2 * FP;
    assert_eq!(game.step(50), Some(Collision::Paddle));
    let mut game = BreakoutGame::new();
    game.status = Status::Playing;
    game.ball_x = FP;
    game.ball_y = FIELD_TOP * FP;
    game.vx = -2 * FP;
    game.vy = 2 * FP;
    assert_eq!(game.step(50), Some(Collision::Brick));
    assert_eq!(game.score, 10);
}

#[test]
fn fastest_ball_cannot_tunnel_through_a_brick_from_below() {
    let mut game = BreakoutGame::new();
    game.status = Status::Playing;
    game.ball_x = 3 * FP;
    game.ball_y = 31 * FP;
    game.vx = 0;
    game.vy = -4 * FP;
    assert_eq!(game.step(50), Some(Collision::Brick));
    assert!(!game.has_brick(16));
    assert_eq!(game.score, 10);
    assert!(game.vy > 0);
    game.step(100);
    assert_eq!(game.score, 10);
}

#[test]
fn side_collision_removes_one_brick_and_reflects_horizontal_velocity() {
    let mut game = BreakoutGame::new();
    game.status = Status::Playing;
    game.bricks = (1 << 1) | (1 << 7);
    game.ball_x = 15 * FP;
    game.ball_y = 14 * FP;
    game.vx = 4 * FP;
    game.vy = 0;
    assert_eq!(game.step(50), Some(Collision::Brick));
    assert_eq!(game.score, 10);
    assert!(!game.has_brick(1) && game.has_brick(7));
    assert!(game.vx < 0);
}

#[test]
fn a_missed_bar_costs_one_life_and_requires_a_new_launch() {
    let mut game = BreakoutGame::new();
    for remaining in [2, 1, 0] {
        game.launch();
        game.ball_x = 5 * FP;
        game.ball_y = 63 * FP;
        game.vy = 4 * FP;
        game.step(50);
        assert_eq!(game.lives, remaining);
        assert_eq!(
            game.status,
            if remaining == 0 {
                Status::GameOver
            } else {
                Status::Serving
            }
        );
        assert_eq!(game.ball_position().1, 57);
        let stopped = game;
        game.step(100);
        assert_eq!(game, stopped);
    }
    game.launch();
    assert_eq!(game.status, Status::GameOver);
}

#[test]
fn last_brick_freezes_for_one_second_then_serves_the_next_level() {
    let mut game = BreakoutGame::new();
    game.status = Status::Playing;
    game.bricks = 1 << 16;
    game.ball_x = 3 * FP;
    game.ball_y = 30 * FP;
    game.vy = -4 * FP;
    game.step(100);
    assert_eq!(game.status, Status::Celebrating { started_ms: 100 });
    game.set_direction(PaddleDirection::Right);
    let frozen = game.ball_position();
    let paddle = game.paddle_x();
    game.launch();
    game.step(1099);
    assert_eq!(game.ball_position(), frozen);
    assert_eq!(game.paddle_x(), paddle + 4);
    game.step(1100);
    assert_eq!((game.level, game.score, game.lives), (2, 10, 3));
    assert_eq!(game.status, Status::Serving);
    assert_eq!(game.bricks.count_ones(), 32);
    assert_eq!((game.vx, game.vy), (0, 0));
}

#[test]
fn endless_levels_cycle_rows_and_cap_speed_without_resetting_score_or_lives() {
    let mut game = BreakoutGame::new();
    for level in 1..=20 {
        game.level = level;
        game.score = 500;
        game.lives = 2;
        game.status = Status::Celebrating { started_ms: 0 };
        game.step(1000);
        assert_eq!(game.rows(), 3 + (level as usize % 3));
        assert_eq!(game.bricks.count_ones(), (game.rows() * 8) as u32);
        assert_eq!((game.score, game.lives), (500, 2));
        assert_eq!(game.speed_fp(), 2 * FP + (level.min(8) as i32) * (FP / 4));
    }
}

#[test]
fn score_and_level_saturate_without_wrapping() {
    let mut game = BreakoutGame::new();
    game.score = u32::MAX;
    game.bricks = 1;
    game.destroy(0, 0);
    assert_eq!(game.score, u32::MAX);
    game.level = u32::MAX;
    game.step(1000);
    assert_eq!(game.level, u32::MAX);
    assert_eq!(game.speed_fp(), 4 * FP);
}

#[test]
fn long_runs_keep_the_ball_out_of_solid_bricks_and_inside_the_playfield() {
    for pattern in 0..3 {
        let mut game = BreakoutGame::new();
        game.level = 9 + pattern;
        game.reset_bricks();
        for tick in 0..2000 {
            if game.status == Status::Serving {
                game.launch();
            }
            game.set_direction(if tick % 40 < 20 {
                PaddleDirection::Left
            } else {
                PaddleDirection::Right
            });
            game.step(tick * 50);
            assert!((1..=107).contains(&game.paddle_x));
            assert!((FP..=125 * FP).contains(&game.ball_x));
            assert!((FIELD_TOP * FP..64 * FP).contains(&game.ball_y));
            assert!(game.brick_at(game.ball_x, game.ball_y).is_none());
            if game.status == Status::GameOver {
                break;
            }
        }
    }
}
