use embassy_rp::{
    adc::{Adc, Async, Channel as AdcChannel},
    gpio::Input,
    rom_data,
};
use embassy_time::{Duration, Timer, Instant};
use embassy_futures::select::{select, Either};
use crate::state::{AppState, STATE};

#[embassy_executor::task]
pub async fn input_task(
    mut adc: Adc<'static, Async>,
    mut joy_x: AdcChannel<'static>,
    mut joy_y: AdcChannel<'static>,
    btn_a: Input<'static>,
    btn_b: Input<'static>,
) {
    let max_songs: u8 = 7;
    let max_arts: u8 = 7;

    let mut rx = STATE.receiver().unwrap();
    let mut current_state: AppState = rx.get().await;

    let mut a_was_pressed: bool = false;
    let mut b_was_pressed: bool = false;
    let mut last_joy_move = Instant::now();

    let update_carousel = |current_id: u8, max_id: u8, joy_val: u16| -> (u8, bool) {
        match joy_val {
            0..=1000 => (current_id.checked_sub(1).unwrap_or(max_id - 1), true),
            3000..=u16::MAX => ((current_id + 1) % max_id, true),
            _ => (current_id, false),
        }
    };

    loop {
        if let Either::First(new_state) = select(rx.changed(), Timer::after(Duration::from_ticks(0))).await {
            current_state = new_state;
        }

        let a_is_pressed: bool = btn_a.is_low();
        let b_is_pressed: bool = btn_b.is_low();

        if a_is_pressed && b_is_pressed {
            Timer::after(Duration::from_secs(2)).await;
            if btn_a.is_low() && btn_b.is_low() {
                rom_data::reset_to_usb_boot(0, 0);
            }
        }

        let a_just_pressed: bool = a_is_pressed && !a_was_pressed;
        let b_just_pressed: bool = b_is_pressed && !b_was_pressed;

        a_was_pressed = a_is_pressed;
        b_was_pressed = b_is_pressed;

        let now = Instant::now();
        let joy_cooldown_ok = now.duration_since(last_joy_move).as_millis() > 300;

        let next_state = match current_state {
            AppState::Menu { song_id, art_id } => {
                let mut new_song_id = song_id;
                let mut new_art_id = art_id;
                
                if joy_cooldown_ok {
                    let x_val = adc.read(&mut joy_x).await.unwrap_or(2048);
                    let y_val = adc.read(&mut joy_y).await.unwrap_or(2048);
                    
                    let (s_id, s_moved) = update_carousel(song_id, max_songs, x_val);
                    let (a_id, a_moved) = update_carousel(art_id, max_arts, y_val);
                    
                    new_song_id = s_id;
                    new_art_id = a_id;
                    
                    if s_moved || a_moved { last_joy_move = now; }
                }

                if b_just_pressed {
                    AppState::Playing { song_id: new_song_id, art_id: new_art_id, paused: false }
                } else {
                    AppState::Menu { song_id: new_song_id, art_id: new_art_id }
                }
            }
            
            AppState::Playing { song_id, art_id, paused } => {
                let mut new_art_id = art_id;
                
                if joy_cooldown_ok {
                    let y_val = adc.read(&mut joy_y).await.unwrap_or(2048);

                    let (a_id, a_moved) = update_carousel(art_id, max_arts, y_val);
                    new_art_id = a_id;
                    
                    if a_moved { last_joy_move = now; }
                }

                if b_just_pressed {
                    AppState::Playing { song_id, art_id: new_art_id, paused: !paused }
                } else if a_just_pressed {
                    AppState::Menu { song_id, art_id: new_art_id }
                } else {
                    AppState::Playing { song_id, art_id: new_art_id, paused }
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