use super::*;

fn time(millis: u64) -> Time {
    Time {
        millis,
        seed: millis + 42,
    }
}
fn controller() -> Controller {
    Controller::new(time(0))
}
fn input(app: &mut Controller, millis: u64, sample: InputSample) -> Effects {
    app.handle(AppEvent::Input(sample), time(millis))
}
fn idle(app: &mut Controller, millis: u64) {
    input(app, millis, InputSample::default());
}
fn tick(app: &mut Controller, millis: u64) {
    app.handle(AppEvent::Tick, time(millis));
}
fn play(app: &mut Controller) -> u64 {
    input(
        app,
        0,
        InputSample {
            b: true,
            ..Default::default()
        },
    );
    app.audio_request().session().unwrap()
}
fn games(app: &mut Controller, game_id: GameId, with_music: bool) {
    app.state = AppState::GamesMenu {
        game_id,
        with_music,
    };
}

#[test]
fn catalog_navigation_visits_every_song_and_wraps_both_ways() {
    let mut app = controller();
    assert_eq!(SONGS.len(), 11);
    for index in 1..=SONGS.len() {
        input(
            &mut app,
            index as u64 * 350,
            InputSample {
                x: 3000,
                ..Default::default()
            },
        );
        assert_eq!(
            app.state(),
            AppState::Menu {
                song_id: (index % SONGS.len()) as u8,
                art_id: 0
            }
        );
    }
    input(
        &mut app,
        4200,
        InputSample {
            x: 1000,
            ..Default::default()
        },
    );
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: TETRIS,
            art_id: 0
        }
    );
}

#[test]
fn catalog_navigation_visits_every_art_and_wraps_both_ways() {
    let mut app = controller();
    assert_eq!(ARTS.len(), 11);
    for index in 1..=ARTS.len() {
        input(
            &mut app,
            index as u64 * 350,
            InputSample {
                y: 3000,
                ..Default::default()
            },
        );
        assert_eq!(
            app.state(),
            AppState::Menu {
                song_id: 0,
                art_id: (index % ARTS.len()) as u8
            }
        );
    }
    input(
        &mut app,
        4200,
        InputSample {
            y: 1000,
            ..Default::default()
        },
    );
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 0,
            art_id: 10
        }
    );
}

#[test]
fn tetris_can_be_selected_and_played_from_the_catalog() {
    let mut app = controller();
    input(
        &mut app,
        350,
        InputSample {
            x: 0,
            ..Default::default()
        },
    );
    input(
        &mut app,
        400,
        InputSample {
            b: true,
            ..Default::default()
        },
    );
    assert!(matches!(
        app.audio_request(),
        AudioRequest::Play {
            song_id: TETRIS,
            repeat: false,
            paused: false,
            ..
        }
    ));
    assert_eq!(SONGS[TETRIS as usize].name, "Tetris");
    assert!(!SONGS[TETRIS as usize].track_a.is_empty());
    assert!(!SONGS[TETRIS as usize].track_b.is_empty());
}

#[test]
fn joystick_has_original_dead_zone_and_shared_strict_cooldown() {
    let mut app = controller();
    input(
        &mut app,
        300,
        InputSample {
            x: 0,
            y: 4095,
            ..Default::default()
        },
    );
    assert_eq!(app.state(), AppState::INITIAL);
    input(
        &mut app,
        301,
        InputSample {
            x: 1001,
            y: 2999,
            ..Default::default()
        },
    );
    assert_eq!(app.state(), AppState::INITIAL);
    input(
        &mut app,
        350,
        InputSample {
            x: 3000,
            y: 1000,
            ..Default::default()
        },
    );
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 1,
            art_id: 10
        }
    );
    input(
        &mut app,
        650,
        InputSample {
            x: 4095,
            ..Default::default()
        },
    );
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 1,
            art_id: 10
        }
    );
}

#[test]
fn pause_and_art_changes_keep_the_audio_session() {
    let mut app = controller();
    let session = play(&mut app);
    idle(&mut app, 50);
    input(
        &mut app,
        350,
        InputSample {
            b: true,
            y: 4095,
            ..Default::default()
        },
    );
    assert_eq!(
        app.state(),
        AppState::Playing {
            song_id: 0,
            art_id: 1,
            paused: true
        }
    );
    assert_eq!(
        app.audio_request(),
        AudioRequest::Play {
            session,
            song_id: 0,
            repeat: false,
            paused: true
        }
    );
    idle(&mut app, 400);
    input(
        &mut app,
        450,
        InputSample {
            b: true,
            ..Default::default()
        },
    );
    assert_eq!(
        app.audio_request(),
        AudioRequest::Play {
            session,
            song_id: 0,
            repeat: false,
            paused: false
        }
    );
}

#[test]
fn simultaneous_a_and_b_preserve_pause_priority_while_playing() {
    let mut app = controller();
    play(&mut app);
    idle(&mut app, 50);
    input(
        &mut app,
        100,
        InputSample {
            a: true,
            b: true,
            ..Default::default()
        },
    );
    assert!(matches!(
        app.state(),
        AppState::Playing { paused: true, .. }
    ));
}

#[test]
fn finishing_audio_uses_the_latest_artwork() {
    let mut app = controller();
    let session = play(&mut app);
    input(
        &mut app,
        350,
        InputSample {
            y: 4095,
            ..Default::default()
        },
    );
    app.handle(AppEvent::AudioFinished { session }, time(400));
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 0,
            art_id: 1
        }
    );
    assert_eq!(app.audio_request(), AudioRequest::Silence);
}

#[test]
fn leaving_and_restarting_the_same_song_rejects_old_completion_and_failure() {
    let mut app = controller();
    let old_session = play(&mut app);
    idle(&mut app, 50);
    input(
        &mut app,
        100,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    assert_eq!(app.audio_request(), AudioRequest::Silence);
    app.handle(
        AppEvent::AudioFinished {
            session: old_session,
        },
        time(110),
    );
    assert_eq!(app.state(), AppState::INITIAL);
    idle(&mut app, 150);
    input(
        &mut app,
        200,
        InputSample {
            b: true,
            ..Default::default()
        },
    );
    assert_ne!(app.audio_request().session(), Some(old_session));
    let state = app.state();
    app.handle(
        AppEvent::AudioFinished {
            session: old_session,
        },
        time(250),
    );
    app.handle(
        AppEvent::AudioFailed {
            session: old_session,
        },
        time(300),
    );
    assert_eq!(app.state(), state);
}

#[test]
fn current_audio_failure_returns_to_menu() {
    let mut app = controller();
    let session = play(&mut app);
    app.handle(AppEvent::AudioFailed { session }, time(50));
    assert_eq!(app.state(), AppState::INITIAL);
}

#[test]
fn long_press_changes_menus_after_two_seconds_and_repeats_as_before() {
    let mut app = controller();
    input(
        &mut app,
        0,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    input(
        &mut app,
        1999,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    assert_eq!(app.state(), AppState::INITIAL);
    input(
        &mut app,
        2000,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    assert_eq!(
        app.state(),
        AppState::GamesMenu {
            game_id: GameId::Snake,
            with_music: true
        }
    );
    input(
        &mut app,
        2050,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    input(
        &mut app,
        4050,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    assert_eq!(app.state(), AppState::INITIAL);
}

#[test]
fn interrupted_hold_does_not_open_games() {
    let mut app = controller();
    input(
        &mut app,
        0,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    idle(&mut app, 1500);
    input(
        &mut app,
        2000,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    input(
        &mut app,
        3999,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    assert_eq!(app.state(), AppState::INITIAL);
}

#[test]
fn triple_click_on_heart_runs_a_timed_easter_egg_without_input() {
    let mut app = controller();
    for ms in [0, 200, 400] {
        input(
            &mut app,
            ms,
            InputSample {
                a: true,
                ..Default::default()
            },
        );
        idle(&mut app, ms + 50);
    }
    assert_eq!(app.state(), AppState::EasterEgg);
    input(
        &mut app,
        500,
        InputSample {
            b: true,
            x: 4095,
            ..Default::default()
        },
    );
    assert_eq!(app.state(), AppState::EasterEgg);
    tick(&mut app, 1900);
    assert_eq!(app.state(), AppState::EasterEgg);
    tick(&mut app, 1950);
    assert_eq!(app.state(), AppState::INITIAL);
}

#[test]
fn triple_click_requires_heart_and_intervals_shorter_than_500_ms() {
    let mut app = controller();
    for ms in [0, 500, 1000] {
        input(
            &mut app,
            ms,
            InputSample {
                a: true,
                ..Default::default()
            },
        );
        idle(&mut app, ms + 50);
    }
    assert_eq!(app.state(), AppState::INITIAL);
    app.state = AppState::Menu {
        song_id: 0,
        art_id: 1,
    };
    app.a_click_count = 0;
    for ms in [2000, 2200, 2400] {
        input(
            &mut app,
            ms,
            InputSample {
                a: true,
                ..Default::default()
            },
        );
        idle(&mut app, ms + 50);
    }
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 0,
            art_id: 1
        }
    );
}

#[test]
fn usb_reset_requires_all_three_buttons_for_two_uninterrupted_seconds() {
    let mut app = controller();
    let buttons = InputSample {
        a: true,
        b: true,
        joy: true,
        ..Default::default()
    };
    assert!(!input(&mut app, 0, buttons).reset_usb);
    assert!(!input(&mut app, 1999, buttons).reset_usb);
    assert!(input(&mut app, 2000, buttons).reset_usb);
    idle(&mut app, 2050);
    assert!(!input(&mut app, 2100, buttons).reset_usb);
}

#[test]
fn games_menu_cycles_and_keeps_start_button_priority() {
    let mut app = controller();
    games(&mut app, GameId::Snake, true);
    input(
        &mut app,
        350,
        InputSample {
            x: 4095,
            ..Default::default()
        },
    );
    assert_eq!(
        app.state(),
        AppState::GamesMenu {
            game_id: GameId::FlappyBird,
            with_music: true
        }
    );
    input(
        &mut app,
        400,
        InputSample {
            a: true,
            b: true,
            ..Default::default()
        },
    );
    assert!(matches!(app.state(), AppState::FlappyBird(_, true)));
    assert!(matches!(
        app.audio_request(),
        AudioRequest::Play {
            song_id: TETRIS,
            repeat: true,
            ..
        }
    ));
}

#[test]
fn game_music_selection_and_silent_games_are_preserved() {
    for (game_id, song_id) in [(GameId::Snake, HINO_NACIONAL), (GameId::FlappyBird, TETRIS)] {
        let mut app = controller();
        games(&mut app, game_id, true);
        input(
            &mut app,
            0,
            InputSample {
                a: true,
                ..Default::default()
            },
        );
        idle(&mut app, 50);
        input(
            &mut app,
            100,
            InputSample {
                b: true,
                ..Default::default()
            },
        );
        assert_eq!(app.audio_request(), AudioRequest::Silence);
        let mut app = controller();
        games(&mut app, game_id, true);
        input(
            &mut app,
            0,
            InputSample {
                b: true,
                ..Default::default()
            },
        );
        assert!(
            matches!(app.audio_request(), AudioRequest::Play { song_id: actual, repeat: true, .. } if actual == song_id)
        );
    }
}

#[test]
fn game_updates_and_restarts_do_not_restart_background_music() {
    let mut app = controller();
    games(&mut app, GameId::FlappyBird, true);
    input(
        &mut app,
        0,
        InputSample {
            b: true,
            ..Default::default()
        },
    );
    let request = app.audio_request();
    tick(&mut app, 450);
    assert_eq!(app.audio_request(), request);
    if let AppState::FlappyBird(mut game, music) = app.state() {
        game.game_over = true;
        app.state = AppState::FlappyBird(game, music);
    }
    idle(&mut app, 500);
    input(
        &mut app,
        550,
        InputSample {
            b: true,
            ..Default::default()
        },
    );
    assert_eq!(app.audio_request(), request);
    tick(&mut app, 950);
    assert!(matches!(
        app.state(),
        AppState::FlappyBird(FlappyGame { bird_y: 2, .. }, true)
    ));
}

#[test]
fn failed_game_music_stays_silent_without_retries() {
    let mut app = controller();
    games(&mut app, GameId::Snake, true);
    input(
        &mut app,
        0,
        InputSample {
            b: true,
            ..Default::default()
        },
    );
    let session = app.audio_request().session().unwrap();
    app.handle(AppEvent::AudioFailed { session }, time(50));
    tick(&mut app, 100);
    assert!(matches!(app.state(), AppState::Snake(_, false)));
    assert_eq!(app.audio_request(), AudioRequest::Silence);
}

#[test]
fn entering_a_game_resets_its_movement_clock() {
    let mut app = controller();
    games(&mut app, GameId::FlappyBird, false);
    input(
        &mut app,
        10000,
        InputSample {
            b: true,
            ..Default::default()
        },
    );
    tick(&mut app, 10400);
    assert!(matches!(
        app.state(),
        AppState::FlappyBird(FlappyGame { bird_y: 2, .. }, false)
    ));
    tick(&mut app, 10450);
    assert!(matches!(
        app.state(),
        AppState::FlappyBird(FlappyGame { bird_y: 3, .. }, false)
    ));
}

#[test]
fn snake_direction_preserves_strict_thresholds_y_priority_and_idle_start() {
    assert_eq!(
        snake_direction(
            InputSample {
                x: 3000,
                y: 1000,
                ..Default::default()
            },
            Direction::Idle
        ),
        Direction::Idle
    );
    assert_eq!(
        snake_direction(
            InputSample {
                x: 4095,
                y: 0,
                ..Default::default()
            },
            Direction::Idle
        ),
        Direction::Left
    );
    let mut app = controller();
    games(&mut app, GameId::Snake, false);
    input(
        &mut app,
        10000,
        InputSample {
            b: true,
            ..Default::default()
        },
    );
    tick(&mut app, 11000);
    assert!(matches!(
        app.state(),
        AppState::Snake(
            SnakeGame {
                head: (2, 2),
                dir: Direction::Idle,
                ..
            },
            false
        )
    ));
    // Make the path deterministic, away from randomly placed obstacles.
    if let AppState::Snake(mut game, music) = app.state() {
        game.obstacles = [(4, 4), (4, 3), (4, 2)];
        app.state = AppState::Snake(game, music);
    }
    input(
        &mut app,
        11050,
        InputSample {
            x: 4095,
            ..Default::default()
        },
    );
    assert!(matches!(
        app.state(),
        AppState::Snake(SnakeGame { head: (2, 1), .. }, false)
    ));
    tick(&mut app, 11550);
    assert!(matches!(
        app.state(),
        AppState::Snake(SnakeGame { head: (2, 1), .. }, false)
    ));
    tick(&mut app, 11600);
    assert!(matches!(
        app.state(),
        AppState::Snake(SnakeGame { head: (2, 0), .. }, false)
    ));
}

#[test]
fn simultaneous_buttons_on_game_over_return_to_menu_instead_of_restarting() {
    let mut app = controller();
    let mut game = FlappyGame::new(42);
    game.game_over = true;
    app.state = AppState::FlappyBird(game, false);
    input(
        &mut app,
        0,
        InputSample {
            a: true,
            b: true,
            ..Default::default()
        },
    );
    assert_eq!(
        app.state(),
        AppState::GamesMenu {
            game_id: GameId::FlappyBird,
            with_music: false
        }
    );
}

#[test]
fn flappy_buttons_are_edges_and_joystick_is_a_level_while_ticks_drive_motion() {
    let mut app = controller();
    games(&mut app, GameId::FlappyBird, false);
    input(
        &mut app,
        0,
        InputSample {
            b: true,
            ..Default::default()
        },
    );
    idle(&mut app, 50);
    input(
        &mut app,
        100,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    input(
        &mut app,
        450,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    assert!(matches!(
        app.state(),
        AppState::FlappyBird(FlappyGame { bird_y: 2, .. }, false)
    ));
    tick(&mut app, 450);
    assert!(matches!(
        app.state(),
        AppState::FlappyBird(FlappyGame { bird_y: 1, .. }, false)
    ));
    // Keeping A held does not queue a second jump.
    input(
        &mut app,
        500,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    tick(&mut app, 900);
    assert!(matches!(
        app.state(),
        AppState::FlappyBird(FlappyGame { bird_y: 2, .. }, false)
    ));
    input(
        &mut app,
        950,
        InputSample {
            joy: true,
            ..Default::default()
        },
    );
    tick(&mut app, 1350);
    assert!(matches!(
        app.state(),
        AppState::FlappyBird(FlappyGame { bird_y: 1, .. }, false)
    ));
}

#[test]
fn switching_games_does_not_reuse_the_previous_movement_clock() {
    let mut app = controller();
    games(&mut app, GameId::Snake, true);
    input(
        &mut app,
        0,
        InputSample {
            b: true,
            ..Default::default()
        },
    );
    let previous_session = app.audio_request().session().unwrap();
    // The same transitions used when leaving a game and selecting its neighbour.
    idle(&mut app, 50);
    input(
        &mut app,
        100,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    input(
        &mut app,
        2100,
        InputSample {
            a: true,
            ..Default::default()
        },
    );
    idle(&mut app, 2150);
    input(
        &mut app,
        2200,
        InputSample {
            x: 4095,
            ..Default::default()
        },
    );
    input(
        &mut app,
        2250,
        InputSample {
            b: true,
            ..Default::default()
        },
    );
    assert_ne!(app.audio_request().session(), Some(previous_session));
    tick(&mut app, 2650);
    assert!(matches!(
        app.state(),
        AppState::FlappyBird(FlappyGame { bird_y: 2, .. }, true)
    ));
    tick(&mut app, 2700);
    assert!(matches!(
        app.state(),
        AppState::FlappyBird(FlappyGame { bird_y: 3, .. }, true)
    ));
}
