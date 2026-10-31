use super::*;
use crate::audio::effects::SoundEffect;
use crate::breakout::game::tests::{about_to_hit_brick, about_to_miss};

fn breakout_state(app: &Controller) -> BreakoutGame {
    match app.state() {
        AppState::Breakout(game, _) => game,
        _ => panic!("expected Breakout"),
    }
}

#[test]
fn breakout_is_third_in_the_menu_and_entry_requires_a_new_b_to_launch() {
    assert_eq!(
        GameId::ALL,
        [GameId::Snake, GameId::FlappyBird, GameId::Breakout]
    );
    let mut app = fixture(AppState::GamesMenu {
        game_id: GameId::FlappyBird,
        with_music: true,
    });
    input(
        &mut app,
        0,
        InputSample {
            x: 4095,
            ..neutral()
        },
    );
    assert!(matches!(
        app.state(),
        AppState::GamesMenu {
            game_id: GameId::Breakout,
            ..
        }
    ));
    tap(
        &mut app,
        40,
        InputSample {
            b: true,
            ..neutral()
        },
    );
    assert_eq!(breakout_state(&app).status, Status::Serving);
    assert!(matches!(app.audio_request(), AudioRequest::Effects { .. }));
    tap(
        &mut app,
        150,
        InputSample {
            b: true,
            ..neutral()
        },
    );
    assert_eq!(breakout_state(&app).status, Status::Playing);
}

#[test]
fn breakout_bar_uses_y_and_moves_only_on_the_physics_tick() {
    let mut app = fixture(AppState::Breakout(BreakoutGame::new(), false));
    input(
        &mut app,
        0,
        InputSample {
            x: 4095,
            ..neutral()
        },
    );
    tick(&mut app, 50);
    assert_eq!(breakout_state(&app).paddle_x(), 54);
    input(&mut app, 60, InputSample { y: 0, ..neutral() });
    assert_eq!(breakout_state(&app).paddle_x(), 54);
    tick(&mut app, 100);
    assert_eq!(breakout_state(&app).paddle_x(), 50);
    input(&mut app, 110, neutral());
    tick(&mut app, 150);
    assert_eq!(breakout_state(&app).paddle_x(), 50);
}

#[test]
fn losing_a_ball_blocks_a_held_launch_without_stopping_the_bar() {
    let mut app = fixture(AppState::Breakout(about_to_miss(3), true));
    let held = InputSample {
        b: true,
        y: 0,
        ..neutral()
    };
    stable(&mut app, 0, held);
    tick(&mut app, 50);
    assert_eq!(
        (breakout_state(&app).lives, breakout_state(&app).status),
        (2, Status::Serving)
    );
    input(&mut app, 60, held);
    tick(&mut app, 100);
    assert_eq!(breakout_state(&app).paddle_x(), 46);
    assert_eq!(breakout_state(&app).status, Status::Serving);
    stable(&mut app, 110, neutral());
    stable(
        &mut app,
        150,
        InputSample {
            b: true,
            ..neutral()
        },
    );
    assert_eq!(breakout_state(&app).status, Status::Playing);
}

#[test]
fn phase_completion_requires_new_launch_and_preserves_score_lives_and_audio_session() {
    let mut app = fixture(AppState::Breakout(about_to_hit_brick(true), true));
    let request = app.audio_request();
    tick(&mut app, 50);
    assert_eq!(
        breakout_state(&app).status,
        Status::Celebrating { started_ms: 50 }
    );
    let b = InputSample {
        b: true,
        ..neutral()
    };
    stable(&mut app, 100, b);
    tick(&mut app, 1050);
    assert_eq!(
        (
            breakout_state(&app).level,
            breakout_state(&app).score,
            breakout_state(&app).lives
        ),
        (2, 10, 3)
    );
    input(&mut app, 1100, b);
    assert_eq!(breakout_state(&app).status, Status::Serving);
    assert_eq!(app.audio_request(), request);
    stable(&mut app, 1200, neutral());
    stable(&mut app, 1300, b);
    assert_eq!(breakout_state(&app).status, Status::Playing);
}

#[test]
fn global_exit_remains_active_across_the_phase_transition_and_a_exits_immediately() {
    let mut app = fixture(AppState::Breakout(about_to_hit_brick(true), true));
    tick(&mut app, 50);
    let joy = InputSample {
        joy: true,
        ..neutral()
    };
    stable(&mut app, 100, joy);
    tick(&mut app, 1050);
    input(&mut app, 2120, joy);
    assert_eq!(
        app.state(),
        AppState::GamesMenu {
            game_id: GameId::Breakout,
            with_music: true
        }
    );
    assert_eq!(app.audio_request(), AudioRequest::Silence);
    let mut app = fixture(AppState::Breakout(about_to_hit_brick(true), false));
    tick(&mut app, 50);
    tap(
        &mut app,
        100,
        InputSample {
            a: true,
            ..neutral()
        },
    );
    assert_eq!(
        app.state(),
        AppState::GamesMenu {
            game_id: GameId::Breakout,
            with_music: false
        }
    );
}

#[test]
fn game_over_restart_resets_every_counter_and_allocates_a_new_effect_session() {
    let mut game = about_to_miss(1);
    game.score = 800;
    game.level = 9;
    let mut app = fixture(AppState::Breakout(game, true));
    let old_session = app.audio_request().session().unwrap();
    tick(&mut app, 50);
    assert_eq!(breakout_state(&app).status, Status::GameOver);
    assert_eq!(app.audio_request(), AudioRequest::Silence);
    tap(
        &mut app,
        100,
        InputSample {
            b: true,
            ..neutral()
        },
    );
    let restarted = breakout_state(&app);
    assert_eq!(
        (
            restarted.score,
            restarted.level,
            restarted.lives,
            restarted.status
        ),
        (0, 1, 3, Status::Serving)
    );
    assert_ne!(app.audio_request().session(), Some(old_session));
    let current = app.state();
    app.handle(
        AppEvent::AudioFailed {
            session: old_session,
        },
        time(200),
    );
    assert_eq!(app.state(), current);
}

#[test]
fn collision_effects_respect_the_sound_option_and_are_tagged_with_the_current_session() {
    for with_sound in [false, true] {
        let mut app = fixture(AppState::Breakout(about_to_hit_brick(false), with_sound));
        let output = app.handle(AppEvent::Tick, time(50));
        assert_eq!(breakout_state(&app).score, 10);
        if with_sound {
            let sound = output.sound.unwrap();
            assert_eq!(sound.effect, SoundEffect::Brick);
            assert_eq!(Some(sound.session), app.audio_request().session());
            app.handle(
                AppEvent::AudioFailed {
                    session: sound.session,
                },
                time(100),
            );
            assert!(matches!(app.state(), AppState::Breakout(_, false)));
            assert_eq!(app.audio_request(), AudioRequest::Silence);
        } else {
            assert!(output.sound.is_none());
            assert_eq!(app.audio_request(), AudioRequest::Silence);
        }
    }
}

fn time(millis: u64) -> Time {
    Time {
        millis,
        seed: millis + 42,
    }
}
fn neutral() -> InputSample {
    InputSample::default()
}
fn input(app: &mut Controller, at: u64, sample: InputSample) -> Effects {
    app.handle(AppEvent::Input(sample), time(at))
}
fn stable(app: &mut Controller, at: u64, sample: InputSample) -> Effects {
    input(app, at, sample);
    input(app, at + 20, sample)
}
fn tap(app: &mut Controller, at: u64, sample: InputSample) {
    stable(app, at, sample);
    stable(app, at + 40, neutral());
}
fn tick(app: &mut Controller, at: u64) {
    app.handle(AppEvent::Tick, time(at));
}
fn fixture(state: AppState) -> Controller {
    let mut app = Controller::new(time(0));
    app.state = state;
    tick(&mut app, 0);
    app
}
fn player() -> Controller {
    let mut app = Controller::new(time(0));
    tap(
        &mut app,
        0,
        InputSample {
            b: true,
            ..neutral()
        },
    );
    app
}
fn triple(app: &mut Controller, at: u64) -> u64 {
    for offset in [0, 100, 200] {
        tap(
            app,
            at + offset,
            InputSample {
                joy: true,
                ..neutral()
            },
        );
    }
    at + 260
}
fn flappy_state(app: &Controller) -> FlappyGame {
    match app.state() {
        AppState::FlappyBird(game, _) => game,
        _ => panic!("expected Flappy"),
    }
}

#[test]
fn catalog_navigation_visits_all_eleven_songs_and_arts_and_wraps() {
    let mut app = Controller::new(time(0));
    assert_eq!((SONGS.len(), ARTS.len()), (11, 11));
    for index in 0..11 {
        input(&mut app, index * 40, neutral());
        input(
            &mut app,
            index * 40 + 10,
            InputSample {
                x: 4095,
                y: 4095,
                ..neutral()
            },
        );
        assert_eq!(
            app.state(),
            AppState::Menu {
                song_id: ((index + 1) % 11) as u8,
                art_id: ((index + 1) % 11) as u8
            }
        );
    }
    input(&mut app, 450, neutral());
    input(
        &mut app,
        460,
        InputSample {
            x: 0,
            y: 0,
            ..neutral()
        },
    );
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 10,
            art_id: 10
        }
    );
}

#[test]
fn tetris_can_be_selected_and_started_without_navigation_delay() {
    let mut app = Controller::new(time(0));
    input(&mut app, 0, InputSample { x: 0, ..neutral() });
    tap(
        &mut app,
        40,
        InputSample {
            b: true,
            ..neutral()
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
}

#[test]
fn menu_axes_repeat_independently_and_enter_immediately() {
    let mut app = Controller::new(time(0));
    input(
        &mut app,
        0,
        InputSample {
            x: 3000,
            ..neutral()
        },
    );
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 1,
            art_id: 0
        }
    );
    input(
        &mut app,
        150,
        InputSample {
            x: 3000,
            y: 1000,
            ..neutral()
        },
    );
    input(
        &mut app,
        349,
        InputSample {
            x: 3000,
            y: 1000,
            ..neutral()
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
        350,
        InputSample {
            x: 3000,
            y: 1000,
            ..neutral()
        },
    );
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 2,
            art_id: 10
        }
    );
    input(
        &mut app,
        500,
        InputSample {
            x: 3000,
            y: 1000,
            ..neutral()
        },
    );
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 3,
            art_id: 9
        }
    );
}

#[test]
fn pause_art_changes_and_resume_keep_the_same_session() {
    let mut app = player();
    let session = app.audio_request().session().unwrap();
    tap(
        &mut app,
        100,
        InputSample {
            b: true,
            ..neutral()
        },
    );
    input(
        &mut app,
        200,
        InputSample {
            y: 4095,
            ..neutral()
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
    tap(
        &mut app,
        250,
        InputSample {
            b: true,
            ..neutral()
        },
    );
    assert!(
        matches!(app.audio_request(), AudioRequest::Play { session: actual, paused: false, .. } if actual == session)
    );
}

#[test]
fn a_returns_immediately_after_debounce_and_holding_it_does_not_change_mode() {
    let mut app = player();
    let a = InputSample {
        a: true,
        ..neutral()
    };
    input(&mut app, 100, a);
    assert!(matches!(app.state(), AppState::Playing { .. }));
    input(&mut app, 120, a);
    assert_eq!(app.state(), AppState::INITIAL);
    input(&mut app, 5000, a);
    assert_eq!(app.state(), AppState::INITIAL);
    assert_eq!(app.audio_request(), AudioRequest::Silence);
}

#[test]
fn joystick_long_press_switches_once_and_restores_the_music_selection() {
    let mut app = fixture(AppState::Menu {
        song_id: 8,
        art_id: 9,
    });
    let joy = InputSample {
        joy: true,
        ..neutral()
    };
    stable(&mut app, 0, joy);
    input(&mut app, 2019, joy);
    assert!(matches!(app.state(), AppState::Menu { .. }));
    input(&mut app, 2020, joy);
    assert_eq!(
        app.state(),
        AppState::GamesMenu {
            game_id: GameId::Snake,
            with_music: true
        }
    );
    input(&mut app, 5000, joy);
    assert!(matches!(app.state(), AppState::GamesMenu { .. }));
    stable(&mut app, 5010, neutral());
    assert!(matches!(
        app.state(),
        AppState::GamesMenu {
            with_music: true,
            ..
        }
    ));
    stable(&mut app, 5100, joy);
    input(&mut app, 7120, joy);
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 8,
            art_id: 9
        }
    );
}

#[test]
fn joystick_long_press_during_playback_opens_games_and_stops_music_only_at_threshold() {
    let mut app = player();
    let request = app.audio_request();
    let joy = InputSample {
        joy: true,
        ..neutral()
    };
    stable(&mut app, 100, joy);
    input(&mut app, 2119, joy);
    assert_eq!(app.audio_request(), request);
    input(&mut app, 2120, joy);
    assert!(matches!(app.state(), AppState::GamesMenu { .. }));
    assert_eq!(app.audio_request(), AudioRequest::Silence);
}

#[test]
fn games_menu_joystick_click_toggles_sound_on_release_and_a_returns() {
    let mut app = fixture(AppState::GamesMenu {
        game_id: GameId::Snake,
        with_music: true,
    });
    stable(
        &mut app,
        0,
        InputSample {
            joy: true,
            ..neutral()
        },
    );
    assert!(matches!(
        app.state(),
        AppState::GamesMenu {
            with_music: true,
            ..
        }
    ));
    stable(&mut app, 40, neutral());
    assert!(matches!(
        app.state(),
        AppState::GamesMenu {
            with_music: false,
            ..
        }
    ));
    tap(
        &mut app,
        100,
        InputSample {
            a: true,
            ..neutral()
        },
    );
    assert_eq!(app.state(), AppState::INITIAL);
}

#[test]
fn game_choice_background_music_and_silent_games_follow_the_new_mapping() {
    for (game_id, song_id) in [(GameId::Snake, HINO_NACIONAL), (GameId::FlappyBird, TETRIS)] {
        for with_music in [false, true] {
            let mut app = fixture(AppState::GamesMenu {
                game_id,
                with_music,
            });
            tap(
                &mut app,
                0,
                InputSample {
                    b: true,
                    ..neutral()
                },
            );
            if with_music {
                assert!(
                    matches!(app.audio_request(), AudioRequest::Play { song_id: actual, repeat: true, .. } if actual == song_id)
                );
            } else {
                assert_eq!(app.audio_request(), AudioRequest::Silence);
            }
        }
    }
    let mut app = fixture(AppState::GamesMenu {
        game_id: GameId::Snake,
        with_music: false,
    });
    input(
        &mut app,
        0,
        InputSample {
            x: 4095,
            ..neutral()
        },
    );
    assert!(matches!(
        app.state(),
        AppState::GamesMenu {
            game_id: GameId::FlappyBird,
            ..
        }
    ));
}

#[test]
fn simultaneous_buttons_are_suppressed_in_menus_player_and_game_over() {
    let mut over = FlappyGame::new(42);
    over.game_over = true;
    for state in [
        AppState::INITIAL,
        AppState::Playing {
            song_id: 0,
            art_id: 0,
            paused: false,
        },
        AppState::GamesMenu {
            game_id: GameId::Snake,
            with_music: true,
        },
        AppState::FlappyBird(over, false),
    ] {
        let mut app = fixture(state);
        stable(
            &mut app,
            0,
            InputSample {
                a: true,
                b: true,
                ..neutral()
            },
        );
        assert_eq!(app.state(), state);
        stable(
            &mut app,
            100,
            InputSample {
                b: true,
                ..neutral()
            },
        );
        assert_eq!(app.state(), state);
    }
}

#[test]
fn held_axes_must_center_before_navigating_the_next_screen() {
    let mut app = Controller::new(time(0));
    input(
        &mut app,
        0,
        InputSample {
            y: 4095,
            ..neutral()
        },
    );
    stable(
        &mut app,
        10,
        InputSample {
            b: true,
            y: 4095,
            ..neutral()
        },
    );
    stable(
        &mut app,
        50,
        InputSample {
            y: 4095,
            ..neutral()
        },
    );
    input(
        &mut app,
        1000,
        InputSample {
            y: 4095,
            ..neutral()
        },
    );
    assert!(matches!(app.state(), AppState::Playing { art_id: 1, .. }));
    input(&mut app, 1010, neutral());
    input(
        &mut app,
        1020,
        InputSample {
            y: 4095,
            ..neutral()
        },
    );
    assert!(matches!(app.state(), AppState::Playing { art_id: 2, .. }));
}

#[test]
fn pending_button_does_not_cross_an_audio_completion_screen_change() {
    let mut app = player();
    let session = app.audio_request().session().unwrap();
    let b = InputSample {
        b: true,
        ..neutral()
    };
    input(&mut app, 100, b);
    app.handle(AppEvent::AudioFinished { session }, time(110));
    input(&mut app, 120, b);
    input(&mut app, 400, b);
    assert_eq!(app.state(), AppState::INITIAL);
    stable(&mut app, 500, neutral());
    stable(&mut app, 550, b);
    assert!(matches!(app.state(), AppState::Playing { .. }));
    assert_ne!(app.audio_request().session(), Some(session));
}

#[test]
fn latest_art_is_kept_on_completion_and_old_sessions_are_ignored() {
    let mut app = player();
    let session = app.audio_request().session().unwrap();
    input(
        &mut app,
        100,
        InputSample {
            y: 4095,
            ..neutral()
        },
    );
    app.handle(AppEvent::AudioFinished { session }, time(150));
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 0,
            art_id: 1
        }
    );
    tap(
        &mut app,
        200,
        InputSample {
            b: true,
            ..neutral()
        },
    );
    let current = app.state();
    app.handle(AppEvent::AudioFinished { session }, time(300));
    app.handle(AppEvent::AudioFailed { session }, time(350));
    assert_eq!(app.state(), current);
}

#[test]
fn current_audio_failure_stops_the_player_and_failed_game_music_does_not_retry() {
    let mut app = player();
    let session = app.audio_request().session().unwrap();
    app.handle(AppEvent::AudioFailed { session }, time(100));
    assert_eq!(app.state(), AppState::INITIAL);
    let mut app = fixture(AppState::Snake(SnakeGame::new(42), true));
    let session = app.audio_request().session().unwrap();
    app.handle(AppEvent::AudioFailed { session }, time(100));
    tick(&mut app, 1000);
    assert!(matches!(app.state(), AppState::Snake(_, false)));
    assert_eq!(app.audio_request(), AudioRequest::Silence);
}

#[test]
fn animation_starts_only_after_the_third_joystick_release_and_returns_at_1500_ms() {
    let mut app = Controller::new(time(0));
    let start = triple(&mut app, 0);
    assert_eq!(app.state(), AppState::EasterEgg);
    tick(&mut app, start + 1499);
    assert_eq!(app.state(), AppState::EasterEgg);
    tick(&mut app, start + 1500);
    assert_eq!(app.state(), AppState::INITIAL);
}

#[test]
fn animation_preserves_playing_or_paused_session_without_audio_commands() {
    for paused in [false, true] {
        let mut app = player();
        if paused {
            tap(
                &mut app,
                100,
                InputSample {
                    b: true,
                    ..neutral()
                },
            );
        }
        let previous_state = app.state();
        let request = app.audio_request();
        let start = triple(&mut app, 200);
        assert_eq!(app.state(), AppState::EasterEgg);
        assert_eq!(app.audio_request(), request);
        // Ordinary buttons are ignored while debouncers keep observing releases.
        tap(
            &mut app,
            start + 100,
            InputSample {
                a: true,
                ..neutral()
            },
        );
        tap(
            &mut app,
            start + 200,
            InputSample {
                b: true,
                ..neutral()
            },
        );
        assert_eq!(app.audio_request(), request);
        tick(&mut app, start + 1500);
        assert_eq!(app.state(), previous_state);
        assert_eq!(app.audio_request(), request);
    }
}

#[test]
fn finishing_or_failing_during_animation_updates_return_state_without_restart() {
    for fail in [false, true] {
        let mut app = player();
        let session = app.audio_request().session().unwrap();
        let start = triple(&mut app, 100);
        let event = if fail {
            AppEvent::AudioFailed { session }
        } else {
            AppEvent::AudioFinished { session }
        };
        app.handle(event, time(start + 100));
        assert_eq!(app.state(), AppState::EasterEgg);
        assert_eq!(app.audio_request(), AudioRequest::Silence);
        tick(&mut app, start + 1500);
        assert_eq!(app.state(), AppState::INITIAL);
        assert_eq!(app.audio_request(), AudioRequest::Silence);
    }
}

#[test]
fn animation_restores_the_previous_music_selection() {
    let mut app = fixture(AppState::Menu {
        song_id: 8,
        art_id: 0,
    });
    let start = triple(&mut app, 0);
    tick(&mut app, start + 1500);
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 8,
            art_id: 0
        }
    );
}

#[test]
fn triple_a_nonheart_and_game_contexts_do_not_activate_the_animation() {
    let mut app = Controller::new(time(0));
    for at in [0, 100, 200] {
        tap(
            &mut app,
            at,
            InputSample {
                a: true,
                ..neutral()
            },
        );
    }
    assert_eq!(app.state(), AppState::INITIAL);
    let mut app = fixture(AppState::Menu {
        song_id: 0,
        art_id: 1,
    });
    triple(&mut app, 0);
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 0,
            art_id: 1
        }
    );
    let mut app = fixture(AppState::GamesMenu {
        game_id: GameId::Snake,
        with_music: true,
    });
    triple(&mut app, 0);
    assert_eq!(
        app.state(),
        AppState::GamesMenu {
            game_id: GameId::Snake,
            with_music: false
        }
    );
}

#[test]
fn art_change_and_screen_change_cancel_partial_triple_sequences() {
    let mut app = player();
    tap(
        &mut app,
        100,
        InputSample {
            joy: true,
            ..neutral()
        },
    );
    tap(
        &mut app,
        200,
        InputSample {
            joy: true,
            ..neutral()
        },
    );
    input(
        &mut app,
        300,
        InputSample {
            y: 4095,
            ..neutral()
        },
    );
    input(&mut app, 310, neutral());
    input(&mut app, 320, InputSample { y: 0, ..neutral() });
    tap(
        &mut app,
        400,
        InputSample {
            joy: true,
            ..neutral()
        },
    );
    assert!(matches!(app.state(), AppState::Playing { art_id: 0, .. }));
    let mut app = Controller::new(time(0));
    tap(
        &mut app,
        0,
        InputSample {
            joy: true,
            ..neutral()
        },
    );
    tap(
        &mut app,
        100,
        InputSample {
            joy: true,
            ..neutral()
        },
    );
    tap(
        &mut app,
        200,
        InputSample {
            b: true,
            ..neutral()
        },
    );
    tap(
        &mut app,
        300,
        InputSample {
            joy: true,
            ..neutral()
        },
    );
    assert!(matches!(app.state(), AppState::Playing { .. }));
}

#[test]
fn usb_reset_is_once_and_remains_armed_across_animation_expiry() {
    let mut app = player();
    let start = triple(&mut app, 100);
    let all = InputSample {
        a: true,
        b: true,
        joy: true,
        ..neutral()
    };
    assert!(!stable(&mut app, start + 100, all).reset_usb);
    assert!(!input(&mut app, start + 1500, all).reset_usb);
    assert!(input(&mut app, start + 2120, all).reset_usb);
    assert!(!input(&mut app, start + 4000, all).reset_usb);
    assert!(matches!(
        app.state(),
        AppState::Playing { paused: false, .. }
    ));
}

#[test]
fn entering_a_game_resets_its_movement_clock_and_start_press_does_not_jump() {
    let mut app = fixture(AppState::GamesMenu {
        game_id: GameId::FlappyBird,
        with_music: false,
    });
    stable(
        &mut app,
        10000,
        InputSample {
            b: true,
            ..neutral()
        },
    );
    tick(&mut app, 10420);
    assert_eq!(flappy_state(&app).bird_y, 2);
    tick(&mut app, 10470);
    assert_eq!(flappy_state(&app).bird_y, 3);
}

#[test]
fn a_and_b_each_jump_once_and_joystick_held_never_auto_jumps() {
    for button in [
        InputSample {
            a: true,
            ..neutral()
        },
        InputSample {
            b: true,
            ..neutral()
        },
        InputSample {
            a: true,
            b: true,
            ..neutral()
        },
    ] {
        let mut app = fixture(AppState::FlappyBird(FlappyGame::new(42), false));
        stable(&mut app, 0, button);
        tick(&mut app, 450);
        assert_eq!(flappy_state(&app).bird_y, 1);
        input(&mut app, 500, button);
        tick(&mut app, 900);
        assert_eq!(flappy_state(&app).bird_y, 2);
    }
    let mut app = fixture(AppState::FlappyBird(FlappyGame::new(42), false));
    let joy = InputSample {
        joy: true,
        ..neutral()
    };
    stable(&mut app, 0, joy);
    tick(&mut app, 450);
    assert_eq!(flappy_state(&app).bird_y, 3);
    input(&mut app, 2020, joy);
    assert!(matches!(
        app.state(),
        AppState::GamesMenu {
            game_id: GameId::FlappyBird,
            ..
        }
    ));
}

#[test]
fn game_over_blocks_held_restart_until_release_and_keeps_background_session() {
    let mut app = fixture(AppState::GamesMenu {
        game_id: GameId::FlappyBird,
        with_music: true,
    });
    let b = InputSample {
        b: true,
        ..neutral()
    };
    stable(&mut app, 0, b);
    let request = app.audio_request();
    if let AppState::FlappyBird(mut game, music) = app.state() {
        game.bird_y = 4;
        app.state = AppState::FlappyBird(game, music);
    }
    tick(&mut app, 450);
    assert!(flappy_state(&app).game_over);
    input(&mut app, 500, b);
    assert!(flappy_state(&app).game_over);
    stable(&mut app, 550, neutral());
    stable(&mut app, 600, b);
    assert_eq!(flappy_state(&app).bird_y, 2);
    assert!(!flappy_state(&app).game_over);
    assert_eq!(app.audio_request(), request);
    tick(&mut app, 1020);
    assert_eq!(flappy_state(&app).bird_y, 2);
}

#[test]
fn snake_keeps_orientation_y_priority_immediate_start_and_tick_timing() {
    let mut game = SnakeGame::new(42);
    game.obstacles = [(4, 4), (4, 3), (4, 2)];
    let mut app = fixture(AppState::Snake(game, false));
    input(
        &mut app,
        0,
        InputSample {
            x: 4095,
            y: 0,
            ..neutral()
        },
    );
    assert!(matches!(
        app.state(),
        AppState::Snake(
            SnakeGame {
                head: (1, 2),
                dir: Direction::Left,
                ..
            },
            false
        )
    ));
    tick(&mut app, 500);
    assert!(matches!(
        app.state(),
        AppState::Snake(SnakeGame { head: (1, 2), .. }, false)
    ));
    tick(&mut app, 550);
    assert!(matches!(
        app.state(),
        AppState::Snake(SnakeGame { head: (0, 2), .. }, false)
    ));
    tap(
        &mut app,
        600,
        InputSample {
            a: true,
            ..neutral()
        },
    );
    assert!(matches!(
        app.state(),
        AppState::GamesMenu {
            game_id: GameId::Snake,
            ..
        }
    ));
}

#[test]
fn sampling_at_ten_ms_does_not_change_the_game_tick_or_advance_physics() {
    assert_eq!(INPUT_INTERVAL_MS, 10);
    assert_eq!(APP_TICK_INTERVAL_MS, 50);
    let mut app = fixture(AppState::FlappyBird(FlappyGame::new(42), false));
    for millis in (0..=1000).step_by(10) {
        input(&mut app, millis, neutral());
    }
    assert_eq!(flappy_state(&app).bird_y, 2);
    tick(&mut app, 1000);
    assert_eq!(flappy_state(&app).bird_y, 3);
}

#[test]
fn game_over_maps_a_to_back_and_b_to_restart_for_both_games() {
    let mut snake = SnakeGame::new(42);
    snake.game_over = true;
    snake.score = 5;
    let mut flappy = FlappyGame::new(42);
    flappy.game_over = true;
    flappy.score = 5;
    for (state, game_id) in [
        (AppState::Snake(snake, false), GameId::Snake),
        (AppState::FlappyBird(flappy, false), GameId::FlappyBird),
    ] {
        let mut app = fixture(state);
        tap(
            &mut app,
            0,
            InputSample {
                a: true,
                ..neutral()
            },
        );
        assert_eq!(
            app.state(),
            AppState::GamesMenu {
                game_id,
                with_music: false
            }
        );
        let mut app = fixture(state);
        tap(
            &mut app,
            0,
            InputSample {
                b: true,
                ..neutral()
            },
        );
        match app.state() {
            AppState::Snake(game, false) => assert!(!game.game_over && game.score == 0),
            AppState::FlappyBird(game, false) => assert!(!game.game_over && game.score == 0),
            _ => panic!("expected a restarted game"),
        }
    }
}

#[test]
fn button_held_during_animation_must_be_released_before_affecting_the_player() {
    let mut app = player();
    let request = app.audio_request();
    let start = triple(&mut app, 100);
    let b = InputSample {
        b: true,
        ..neutral()
    };
    stable(&mut app, start + 1400, b);
    input(&mut app, start + 1500, b);
    input(&mut app, start + 2000, b);
    assert_eq!(app.audio_request(), request);
    stable(&mut app, start + 2100, neutral());
    tap(&mut app, start + 2200, b);
    assert!(matches!(
        app.state(),
        AppState::Playing { paused: true, .. }
    ));
    assert_eq!(app.audio_request().session(), request.session());
}

#[test]
fn periodic_tick_is_requested_only_for_active_gameplay_and_animations() {
    let mut app = Controller::new(time(0));
    assert!(!app.needs_tick());

    let start = triple(&mut app, 0);
    assert!(app.needs_tick());
    tick(&mut app, start + 1500);
    assert!(!app.needs_tick());

    let mut snake = SnakeGame::new(42);
    let app = fixture(AppState::Snake(snake, false));
    assert!(!app.needs_tick());
    snake.dir = Direction::Right;
    let app = fixture(AppState::Snake(snake, false));
    assert!(app.needs_tick());
    snake.game_over = true;
    let app = fixture(AppState::Snake(snake, false));
    assert!(!app.needs_tick());

    let mut flappy = FlappyGame::new(42);
    let app = fixture(AppState::FlappyBird(flappy, false));
    assert!(app.needs_tick());
    flappy.game_over = true;
    let app = fixture(AppState::FlappyBird(flappy, false));
    assert!(!app.needs_tick());

    let mut breakout = BreakoutGame::new();
    let app = fixture(AppState::Breakout(breakout, false));
    assert!(app.needs_tick());
    breakout.status = Status::GameOver;
    let app = fixture(AppState::Breakout(breakout, false));
    assert!(!app.needs_tick());
}

#[test]
fn first_joystick_push_works_after_non_input_screen_transition_without_neutral_samples() {
    let mut app = player();
    let session = app.audio_request().session().unwrap();
    app.handle(AppEvent::AudioFinished { session }, time(1000));
    input(&mut app, 2000, InputSample { x: 3500, ..neutral() });
    assert_eq!(
        app.state(),
        AppState::Menu {
            song_id: 1,
            art_id: 0
        }
    );
}
