use crate::flappy::game::FlappyGame;
use crate::snake::game::SnakeGame;
use crate::state::{AppState, STATE};
use embassy_futures::select::{Either, select};
use embassy_rp::adc::{Adc, Async, Channel as AdcChannel};
use embassy_rp::gpio::Input;
use embassy_rp::rom_data;
use embassy_time::{Duration, Instant, Timer};

#[embassy_executor::task]
pub async fn input_task(
    mut adc: Adc<'static, Async>,
    mut joy_x: AdcChannel<'static>,
    mut joy_y: AdcChannel<'static>,
    btn_a: Input<'static>,
    btn_b: Input<'static>,
    btn_joy: Input<'static>,
) {
    let max_songs = 10;
    let max_arts = 11;
    let max_games = 2;

    let mut rx = STATE.receiver().unwrap();
    let mut current_state = rx.get().await;

    let mut a_was_pressed = false;
    let mut b_was_pressed = false;

    let mut last_joy_move = Instant::now();
    let mut last_game_move = Instant::now();

    let mut a_hold_start: Option<Instant> = None;

    let update_carousel = |current_id: u8, max_id: u8, joy_val: u16| -> (u8, bool) {
        match joy_val {
            0..=1000 => ((current_id + max_id - 1) % max_id, true),
            3000..=u16::MAX => ((current_id + 1) % max_id, true),
            _ => (current_id, false),
        }
    };

    let mut a_click_count = 0;
    let mut last_a_click = Instant::now();
    let mut easter_egg_start: Option<Instant> = None;
    let mut reset_hold_start: Option<Instant> = None;

    loop {
        if let Either::First(new_state) =
            select(rx.changed(), Timer::after(Duration::from_ticks(0))).await
        {
            current_state = new_state;
        }

        let a_is_pressed = btn_a.is_low();
        let b_is_pressed = btn_b.is_low();
        let joy_is_pressed = btn_joy.is_low();
        let now = Instant::now();

        if let Some(start_time) = easter_egg_start {
            if now.duration_since(start_time).as_millis() > 1500 {
                easter_egg_start = None;

                let next_state = AppState::Menu {
                    song_id: 0,
                    art_id: 0,
                };
                STATE.sender().send(next_state);
                current_state = next_state;
            }

            Timer::after(Duration::from_millis(50)).await;
            continue;
        }

        // RESET USB
        if a_is_pressed && b_is_pressed && joy_is_pressed {
            if now
                .duration_since(*reset_hold_start.get_or_insert(now))
                .as_secs()
                >= 2
            {
                rom_data::reset_to_usb_boot(0, 0);
            }
        } else {
            reset_hold_start = None;
        }

        let mut override_state: Option<AppState> = None;

        match (a_is_pressed, b_is_pressed, joy_is_pressed) {
            (true, false, false) => {
                if now
                    .duration_since(*a_hold_start.get_or_insert(now))
                    .as_secs()
                    >= 2
                {
                    override_state = Some(match current_state {
                        AppState::Menu { .. } | AppState::Playing { .. } => AppState::GamesMenu {
                            game_id: 0,
                            with_music: true,
                        },

                        AppState::GamesMenu { .. } => AppState::Menu {
                            song_id: 0,
                            art_id: 0,
                        },

                        AppState::Snake(_, m) => AppState::GamesMenu {
                            game_id: 0,
                            with_music: m,
                        },
                        AppState::FlappyBird(_, m) => AppState::GamesMenu {
                            game_id: 1,
                            with_music: m,
                        },

                        _ => current_state,
                    });
                    a_hold_start = None;
                }
            }
            _ => {
                a_hold_start = None;
            }
        }

        let a_just_pressed = a_is_pressed && !a_was_pressed;
        let b_just_pressed = b_is_pressed && !b_was_pressed;

        a_was_pressed = a_is_pressed;
        b_was_pressed = b_is_pressed;

        if a_just_pressed {
            a_click_count = if now.duration_since(last_a_click).as_millis() < 500 {
                a_click_count + 1
            } else {
                1
            };
            last_a_click = now;
        } else if !a_is_pressed && now.duration_since(last_a_click).as_millis() > 500 {
            a_click_count = 0;
        }

        let is_heart = matches!(
            current_state,
            AppState::Menu { art_id: 0, .. } | AppState::Playing { art_id: 0, .. }
        );

        if is_heart && a_click_count == 3 {
            a_click_count = 0;
            easter_egg_start = Some(now);

            let next_state = AppState::EasterEgg;
            STATE.sender().send(next_state);
            current_state = next_state;

            Timer::after(Duration::from_millis(50)).await;
            continue;
        }

        let joy_cooldown_ok = now.duration_since(last_joy_move).as_millis() > 300;

        let next_state = if let Some(state) = override_state {
            state
        } else {
            match current_state {
                AppState::Menu { song_id, art_id } => {
                    let (new_song_id, new_art_id) = if joy_cooldown_ok {
                        let x_val = adc.read(&mut joy_x).await.unwrap_or(2048);
                        let y_val = adc.read(&mut joy_y).await.unwrap_or(2048);

                        let (s_id, s_moved) = update_carousel(song_id, max_songs, x_val);
                        let (a_id, a_moved) = update_carousel(art_id, max_arts, y_val);

                        if s_moved || a_moved {
                            last_joy_move = now;
                        }
                        (s_id, a_id)
                    } else {
                        (song_id, art_id)
                    };

                    if b_just_pressed {
                        AppState::Playing {
                            song_id: new_song_id,
                            art_id: new_art_id,
                            paused: false,
                        }
                    } else {
                        AppState::Menu {
                            song_id: new_song_id,
                            art_id: new_art_id,
                        }
                    }
                }

                AppState::Playing {
                    song_id,
                    art_id,
                    paused,
                } => {
                    let new_art_id = if joy_cooldown_ok {
                        let y_val = adc.read(&mut joy_y).await.unwrap_or(2048);
                        let (a_id, a_moved) = update_carousel(art_id, max_arts, y_val);
                        if a_moved {
                            last_joy_move = now;
                        }
                        a_id
                    } else {
                        art_id
                    };

                    match (b_just_pressed, a_just_pressed) {
                        (true, _) => AppState::Playing {
                            song_id,
                            art_id: new_art_id,
                            paused: !paused,
                        },
                        (false, true) => AppState::Menu {
                            song_id,
                            art_id: new_art_id,
                        },
                        _ => AppState::Playing {
                            song_id,
                            art_id: new_art_id,
                            paused,
                        },
                    }
                }

                AppState::GamesMenu {
                    game_id,
                    with_music,
                } => {
                    let new_game_id = if joy_cooldown_ok {
                        // Move no eixo X para escolher o jogo
                        let x_val = adc.read(&mut joy_x).await.unwrap_or(2048);
                        let (g_id, g_moved) = update_carousel(game_id, max_games, x_val);

                        if g_moved {
                            last_joy_move = now;
                        }
                        g_id
                    } else {
                        game_id
                    };

                    if b_just_pressed {
                        match new_game_id {
                            0 => AppState::Snake(SnakeGame::new(now.as_ticks()), with_music),
                            1 => AppState::FlappyBird(FlappyGame::new(now.as_ticks()), with_music),
                            _ => AppState::GamesMenu {
                                game_id: new_game_id,
                                with_music,
                            },
                        }
                    } else if a_just_pressed {
                        AppState::GamesMenu {
                            game_id: new_game_id,
                            with_music: !with_music,
                        }
                    } else {
                        AppState::GamesMenu {
                            game_id: new_game_id,
                            with_music,
                        }
                    }
                }

                AppState::Snake(mut game, with_music) => {
                    if game.game_over {
                        match (a_just_pressed, b_just_pressed) {
                            (true, _) => AppState::GamesMenu {
                                game_id: 0,
                                with_music,
                            }, // Volta pro menu de jogos
                            (false, true) => {
                                AppState::Snake(SnakeGame::new(now.as_ticks()), with_music)
                            } // Joga de novo
                            _ => AppState::Snake(game, with_music),
                        }
                    } else {
                        let x_val = adc.read(&mut joy_x).await.unwrap_or(2048);
                        let y_val = adc.read(&mut joy_y).await.unwrap_or(2048);

                        let was_idle = game.dir == 4;
                        let time_elapsed = now.duration_since(last_game_move).as_millis() as u32;

                        game.input(x_val, y_val);

                        if game.dir != 4 && (was_idle || time_elapsed > game.speed_ms) {
                            game.step(now.as_ticks());
                            last_game_move = now;
                        }

                        AppState::Snake(game, with_music)
                    }
                }

                AppState::FlappyBird(mut game, with_music) => {
                    if game.game_over {
                        match (a_just_pressed, b_just_pressed) {
                            (true, _) => AppState::GamesMenu {
                                game_id: 1,
                                with_music,
                            }, // Volta pro menu de jogos
                            (false, true) => {
                                AppState::FlappyBird(FlappyGame::new(now.as_ticks()), with_music)
                            } // Joga de novo
                            _ => AppState::FlappyBird(game, with_music),
                        }
                    } else {
                        game.input(a_just_pressed, b_just_pressed, joy_is_pressed);

                        let time_elapsed = now.duration_since(last_game_move).as_millis() as u32;

                        if time_elapsed > game.speed_ms {
                            game.step(now.as_ticks());
                            last_game_move = now;
                        }

                        AppState::FlappyBird(game, with_music)
                    }
                }

                AppState::EasterEgg => current_state,
            }
        };

        if next_state != current_state {
            STATE.sender().send(next_state);
            current_state = next_state;
        }

        Timer::after(Duration::from_millis(50)).await;
    }
}
