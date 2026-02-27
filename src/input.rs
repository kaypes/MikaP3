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
    let max_songs: u8 = 10;
    let max_arts: u8 = 11;

    let mut rx = STATE.receiver().unwrap();
    let mut current_state = rx.get().await;

    let mut a_was_pressed: bool = false;
    let mut b_was_pressed: bool = false;

    let mut last_joy_move = Instant::now();
    let mut last_snake_move = Instant::now();

    let mut a_hold_start: Option<Instant> = None;
    let mut ab_hold_start: Option<Instant> = None;

    let update_carousel = |current_id: u8, max_id: u8, joy_val: u16| -> (u8, bool) {
        match joy_val {
            0..=1000 => (current_id.checked_sub(1).unwrap_or(max_id - 1), true),
            3000..=u16::MAX => ((current_id + 1) % max_id, true),
            _ => (current_id, false),
        }
    };

    loop {
        if let Either::First(new_state) =
            select(rx.changed(), Timer::after(Duration::from_ticks(0))).await {
            current_state = new_state;
        }

        let a_is_pressed: bool = btn_a.is_low();
        let b_is_pressed: bool = btn_b.is_low();
        let joy_is_pressed: bool = btn_joy.is_low();
        let now = Instant::now();

        if a_is_pressed && b_is_pressed && joy_is_pressed {
            Timer::after(Duration::from_secs(2)).await;

            if btn_a.is_low() && btn_b.is_low() && btn_joy.is_low() {
                rom_data::reset_to_usb_boot(0, 0);
            }
        }

        let mut trigger_snake_with_music: bool = false;
        let mut trigger_snake_no_music: bool = false;
        let mut exit_snake: bool = false;

        if a_is_pressed && b_is_pressed && !joy_is_pressed {
            a_hold_start = None;
            
            if ab_hold_start.is_none() {
                ab_hold_start = Some(now);
            } else if now.duration_since(ab_hold_start.unwrap()).as_secs() >= 2 {
                trigger_snake_with_music = true;
                ab_hold_start = None;
            }
        } else if a_is_pressed && !b_is_pressed && !joy_is_pressed {
            ab_hold_start = None;

            if a_hold_start.is_none() {
                a_hold_start = Some(now);
            } else if now.duration_since(a_hold_start.unwrap()).as_secs() >= 2 {
                if let AppState::Snake(_, _) = current_state {
                    exit_snake = true;
                } else {
                    trigger_snake_no_music = true;
                }

                a_hold_start = None;
            }
        } else {
            a_hold_start = None;
            ab_hold_start = None;
        }

        let a_just_pressed: bool = a_is_pressed && !a_was_pressed;
        let b_just_pressed: bool = b_is_pressed && !b_was_pressed;
      
        a_was_pressed = a_is_pressed;
        b_was_pressed = b_is_pressed;

        let joy_cooldown_ok: bool = now.duration_since(last_joy_move).as_millis() > 300;

        let next_state: AppState = match (exit_snake, trigger_snake_with_music, trigger_snake_no_music) {
            (true, _, _) => AppState::Menu { song_id: 0, art_id: 0 },
            (_, true, _) => AppState::Snake(SnakeGame::new(now.as_ticks()), true),
            (_, _, true) => AppState::Snake(SnakeGame::new(now.as_ticks()), false),
            _ => match current_state {
                    AppState::Menu { song_id, art_id } => {
                        let mut new_song_id: u8 = song_id;
                        let mut new_art_id: u8 = art_id;
        
                        if joy_cooldown_ok {
                            let x_val: u16 = adc.read(&mut joy_x).await.unwrap_or(2048);
                            let y_val: u16 = adc.read(&mut joy_y).await.unwrap_or(2048);
        
                            let (s_id, s_moved) = update_carousel(song_id, max_songs, x_val);
                            let (a_id, a_moved) = update_carousel(art_id, max_arts, y_val);
        
                            new_song_id = s_id;
                            new_art_id = a_id;
                           
                            if s_moved || a_moved {
                                last_joy_move = now;
                            }
                        }
        
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
                        let mut new_art_id: u8 = art_id;
        
                        if joy_cooldown_ok {
                            let y_val: u16 = adc.read(&mut joy_y).await.unwrap_or(2048);
                            let (a_id, a_moved) = update_carousel(art_id, max_arts, y_val);
                            
                            new_art_id = a_id;
                            
                            if a_moved {
                                last_joy_move = now;
                            }
                        }
        
                        if b_just_pressed {
                            AppState::Playing {
                                song_id,
                                art_id: new_art_id,
                                paused: !paused,
                            }
                        } else if a_just_pressed {
                            AppState::Menu {
                                song_id,
                                art_id: new_art_id,
                            }
                        } else {
                            AppState::Playing {
                                song_id,
                                art_id: new_art_id,
                                paused,
                            }
                        }
                    }
        
                    AppState::Snake(mut game, with_music) => {
                        if game.game_over {
                            if a_just_pressed {
                                AppState::Menu { song_id: 0, art_id: 0 }
                            } else if b_just_pressed {
                                AppState::Snake(SnakeGame::new(now.as_ticks()), with_music)
                            } else {
                                AppState::Snake(game, with_music)
                            }
                        } else {
                            let x_val: u16 = adc.read(&mut joy_x).await.unwrap_or(2048);
                            let y_val: u16 = adc.read(&mut joy_y).await.unwrap_or(2048);

                            let was_idle: bool = game.dir == 4;
                            let time_elapsed: u32 = now.duration_since(last_snake_move).as_millis() as u32;

                            game.input(x_val, y_val);

                            if game.dir != 4 && (was_idle || time_elapsed > game.speed_ms) {
                                game.step(now.as_ticks());
                                last_snake_move = now;    
                            }
        
                            AppState::Snake(game, with_music)
                        }
                    }
                }
            };

        if next_state != current_state {
            STATE.sender().send(next_state);
            current_state = next_state;
        }

        Timer::after(Duration::from_millis(50)).await;
    }
}