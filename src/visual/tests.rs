use super::*;
use crate::breakout::game::{BreakoutGame, Status};
use crate::flappy::game::FlappyGame;
use crate::snake::game::SnakeGame;

#[test]
fn breakout_leds_are_dark_except_for_one_second_of_timed_celebration() {
    let mut renderer = MatrixRenderer::default();
    let mut game = BreakoutGame::new();
    assert_eq!(
        renderer.update(AppState::Breakout(game, true), 0).frame,
        Some([(0, 0, 0); 25])
    );
    game.status = Status::Celebrating { started_ms: 100 };
    let first = renderer.update(AppState::Breakout(game, true), 100);
    assert!(first.frame.unwrap().iter().all(|&pixel| pixel != (0, 0, 0)));
    assert_eq!(first.next_wake_ms, Some(200));
    let second = renderer.update(AppState::Breakout(game, true), 200);
    assert_ne!(first.frame, second.frame);
    assert_eq!(second.next_wake_ms, Some(300));
    let ended = renderer.update(AppState::Breakout(game, true), 1100);
    assert_eq!(ended.frame, Some([(0, 0, 0); 25]));
    assert_eq!(ended.next_wake_ms, None);
    let menu = renderer.update(
        AppState::GamesMenu {
            game_id: GameId::Breakout,
            with_music: true,
        },
        1110,
    );
    assert_eq!(menu.frame, Some(sprite_frame(&BREAKOUT_ICON)));
}

#[test]
fn oled_cadence_caps_breakout_at_twenty_fps_and_never_delays_other_screens() {
    let mut cadence = OledCadence::default();
    let view = DisplayView::Breakout(BreakoutGame::new());
    assert_eq!(cadence.wait_until(view, 0), None);
    cadence.drawn(view, 0);
    assert_eq!(cadence.wait_until(view, 1), Some(50));
    assert_eq!(cadence.wait_until(view, 49), Some(50));
    assert_eq!(cadence.wait_until(view, 50), None);
    cadence.drawn(view, 50);
    assert_eq!(cadence.wait_until(view, 51), Some(100));
    let menu = DisplayView::from(AppState::INITIAL);
    assert_eq!(cadence.wait_until(menu, 51), None);
    cadence.drawn(menu, 51);
    assert_eq!(cadence.wait_until(view, 52), None);
}

#[test]
fn control_help_fits_two_rows_of_the_oled_in_every_context() {
    let mut views = std::vec![
        DisplayView::GamesMenu {
            game: GameId::Snake,
            with_music: true
        },
        DisplayView::GamesMenu {
            game: GameId::FlappyBird,
            with_music: false
        },
        DisplayView::EasterEgg,
    ];
    for art in ARTS {
        views.push(DisplayView::Menu {
            song: SONGS[0].name,
            art: art.name,
        });
        views.push(DisplayView::Playing {
            song: SONGS[0].name,
            art: art.name,
            paused: false,
        });
    }
    for game in GameId::ALL {
        for game_over in [false, true] {
            views.push(DisplayView::Score {
                game,
                score: 0,
                game_over,
            });
        }
    }
    for view in views {
        for hint in view.control_hints() {
            assert!(hint.is_ascii());
            assert!(hint.len() * 6 <= 128, "hint does not fit: {hint}");
        }
    }
}

#[test]
fn sprite_palette_and_row_order_preserve_existing_brightness() {
    let frame = sprite_frame(&["RGBYP", "CWOL.", ".....", ".....", "....."]);
    assert_eq!(
        &frame[..10],
        &[
            (10, 0, 0),
            (0, 5, 0),
            (0, 0, 3),
            (10, 5, 0),
            (10, 0, 3),
            (0, 5, 3),
            (10, 5, 3),
            (10, 2, 0),
            (5, 3, 0),
            (0, 0, 0),
        ]
    );
    assert!(frame[10..].iter().all(|&pixel| pixel == (0, 0, 0)));
}

#[test]
fn every_catalog_sprite_icon_and_animation_frame_is_a_valid_five_by_five_grid() {
    for sprite in ARTS
        .iter()
        .map(|art| &art.sprite)
        .chain(CORACAO_ANIMATION.iter())
        .chain([&SNAKE_ICON, &FLAPPY_ICON])
    {
        for row in sprite {
            assert_eq!(row.len(), 5);
            assert!(row.bytes().all(|symbol| b".RGBYPCWOL".contains(&symbol)));
        }
    }
}

#[test]
fn matrix_suppresses_identical_frames_across_song_pause_and_score_changes() {
    let mut renderer = MatrixRenderer::default();
    assert!(renderer.update(AppState::INITIAL, 0).frame.is_some());
    assert!(renderer.update(AppState::INITIAL, 50).frame.is_none());
    assert!(
        renderer
            .update(
                AppState::Playing {
                    song_id: 10,
                    art_id: 0,
                    paused: true
                },
                100
            )
            .frame
            .is_none()
    );
    let mut game = SnakeGame::new(42);
    assert!(
        renderer
            .update(AppState::Snake(game, false), 150)
            .frame
            .is_some()
    );
    game.score += 1;
    assert!(
        renderer
            .update(AppState::Snake(game, true), 200)
            .frame
            .is_none()
    );
}

#[test]
fn animation_uses_absolute_deadlines_and_holds_its_last_frame() {
    let mut renderer = MatrixRenderer::default();
    for (index, sprite) in CORACAO_ANIMATION.iter().enumerate() {
        let now = 100 + index as u64 * 150;
        let update = renderer.update(AppState::EasterEgg, now);
        assert_eq!(update.frame, Some(sprite_frame(sprite)));
        let deadline = if index + 1 < CORACAO_ANIMATION.len() {
            Some(now + 150)
        } else {
            None
        };
        assert_eq!(update.next_wake_ms, deadline);
        assert!(
            renderer
                .update(AppState::EasterEgg, now + 149)
                .frame
                .is_none()
        );
    }
    let last = renderer.update(AppState::EasterEgg, 1500);
    assert!(last.frame.is_none() && last.next_wake_ms.is_none());
}

#[test]
fn delayed_animation_wake_skips_expired_frames() {
    let mut renderer = MatrixRenderer::default();
    renderer.update(AppState::EasterEgg, 100);
    let update = renderer.update(AppState::EasterEgg, 725);
    assert_eq!(update.frame, Some(sprite_frame(&CORACAO_ANIMATION[4])));
    assert_eq!(update.next_wake_ms, Some(850));
}

#[test]
fn exiting_an_animation_restores_the_same_art_id_and_reentry_starts_at_zero() {
    let mut renderer = MatrixRenderer::default();
    let heart = renderer.update(AppState::INITIAL, 0).frame.unwrap();
    renderer.update(AppState::EasterEgg, 50);
    renderer.update(AppState::EasterEgg, 200);
    assert_eq!(renderer.update(AppState::INITIAL, 210).frame, Some(heart));
    let restarted = renderer.update(AppState::EasterEgg, 300);
    assert_eq!(restarted.frame, Some(sprite_frame(&CORACAO_ANIMATION[0])));
    assert_eq!(restarted.next_wake_ms, Some(450));
}

#[test]
fn games_menu_uses_the_original_icons() {
    for (game_id, icon) in [
        (GameId::Snake, SNAKE_ICON),
        (GameId::FlappyBird, FLAPPY_ICON),
    ] {
        let mut renderer = MatrixRenderer::default();
        let update = renderer.update(
            AppState::GamesMenu {
                game_id,
                with_music: false,
            },
            0,
        );
        assert_eq!(update.frame, Some(sprite_frame(&icon)));
    }
}

#[test]
fn snake_frame_preserves_head_obstacle_fruit_and_game_over_colors() {
    let mut game = SnakeGame::new(42);
    game.head = (2, 2);
    game.obstacles = [(0, 0), (1, 0), (2, 0)];
    game.fruit = Some((4, 4));
    let frame = crate::snake::render::draw_frame(&game);
    assert_eq!(frame[12], (10, 10, 0));
    assert_eq!(&frame[..3], &[(0, 0, 10); 3]);
    assert_eq!(frame[24], (0, 10, 0));
    assert_eq!(frame[20], (0, 0, 0));
    game.game_over = true;
    assert_eq!(crate::snake::render::draw_frame(&game), [(2, 0, 0); 25]);
}

#[test]
fn flappy_frame_preserves_pipe_bird_and_crash_colors() {
    let mut game = FlappyGame::new(42);
    game.pipe_x = 4;
    game.pipe_gap = 2;
    let frame = crate::flappy::render::draw_frame(&game);
    assert_eq!(frame[11], (10, 5, 0));
    for y in 0..5 {
        assert_eq!(
            frame[y * 5 + 4],
            if y == 1 || y == 2 {
                (0, 0, 0)
            } else {
                (0, 5, 0)
            }
        );
    }
    game.game_over = true;
    assert_eq!(crate::flappy::render::draw_frame(&game)[11], (10, 0, 0));
}

#[test]
fn oled_gets_names_from_the_same_catalogs_as_playback_and_leds() {
    assert_eq!(
        DisplayView::from(AppState::Playing {
            song_id: 10,
            art_id: 10,
            paused: true
        }),
        DisplayView::Playing {
            song: SONGS[10].name,
            art: ARTS[10].name,
            paused: true
        }
    );
    assert_eq!(SONGS[10].name, "Tetris");
}

#[test]
fn oled_ignores_game_motion_but_tracks_score_game_over_and_game_identity() {
    let mut game = FlappyGame::new(42);
    let first = DisplayView::from(AppState::FlappyBird(game, true));
    game.bird_y += 1;
    game.pipe_x -= 1;
    assert_eq!(DisplayView::from(AppState::FlappyBird(game, false)), first);
    game.score += 1;
    assert_ne!(DisplayView::from(AppState::FlappyBird(game, false)), first);
    game.score = 0;
    game.game_over = true;
    assert_ne!(DisplayView::from(AppState::FlappyBird(game, false)), first);
    assert_ne!(
        DisplayView::from(AppState::Snake(SnakeGame::new(42), true)),
        first
    );
}

#[test]
fn leds_change_filter_skips_non_visual_updates_and_breakout_gameplay_ticks() {
    assert_eq!(DisplayView::INITIAL, DisplayView::from(AppState::INITIAL));
    assert!(!leds_relevant_change(
        AppState::Menu {
            song_id: 0,
            art_id: 2
        },
        AppState::Playing {
            song_id: 5,
            art_id: 2,
            paused: true
        }
    ));
    assert!(leds_relevant_change(
        AppState::Menu {
            song_id: 0,
            art_id: 2
        },
        AppState::Menu {
            song_id: 0,
            art_id: 3
        }
    ));

    let serving = BreakoutGame::new();
    let mut playing = serving;
    playing.launch();
    playing.step(50);
    assert!(!leds_relevant_change(
        AppState::Breakout(serving, true),
        AppState::Breakout(playing, true)
    ));
    let mut celebrating = playing;
    celebrating.status = Status::Celebrating { started_ms: 100 };
    assert!(leds_relevant_change(
        AppState::Breakout(playing, true),
        AppState::Breakout(celebrating, true)
    ));

    let mut aimed = serving;
    aimed.set_direction(crate::breakout::game::PaddleDirection::Left);
    assert!(!oled_relevant_change(
        AppState::Breakout(serving, true),
        AppState::Breakout(aimed, true)
    ));
    let mut moved = aimed;
    moved.step(50);
    assert!(oled_relevant_change(
        AppState::Breakout(aimed, true),
        AppState::Breakout(moved, true)
    ));
}
