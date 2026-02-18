use embassy_rp::adc::{Adc, Async, Channel as AdcChannel};
use embassy_rp::gpio::Input;
use embassy_time::{Duration, Timer};
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

    loop {
        if let Either::First(new_state) = select(rx.changed(), Timer::after(Duration::from_ticks(0))).await {
            current_state = new_state;
        }

        let x_val: u16 = adc.read(&mut joy_x).await.unwrap_or(2048);
        let y_val: u16 = adc.read(&mut joy_y).await.unwrap_or(2048);
        
        let a_is_pressed: bool = btn_a.is_low();
        let b_is_pressed: bool = btn_b.is_low();

        let a_just_pressed: bool = a_is_pressed && !a_was_pressed;
        let b_just_pressed: bool = b_is_pressed && !b_was_pressed;

        a_was_pressed = a_is_pressed;
        b_was_pressed = b_is_pressed;

        let next_state = match current_state {
            
            AppState::Menu { mut song_id, mut art_id } => {
                if x_val < 1000 { 
                    if song_id == 0 { song_id = max_songs - 1; } else { song_id -= 1; }
                    Timer::after(Duration::from_millis(300)).await;
                } else if x_val > 3000 {
                    if song_id == max_songs - 1 { song_id = 0; } else { song_id += 1; }
                    Timer::after(Duration::from_millis(300)).await;
                }

                if y_val < 1000 {
                    if art_id == 0 { art_id = max_arts - 1; } else { art_id -= 1; }
                    Timer::after(Duration::from_millis(300)).await;
                } else if y_val > 3000 {
                    if art_id == max_arts - 1 { art_id = 0; } else { art_id += 1; }
                    Timer::after(Duration::from_millis(300)).await;
                }

                if b_just_pressed {
                    AppState::Playing { song_id, art_id, paused: false }
                } else {
                    AppState::Menu { song_id, art_id }
                }
            }
            
            AppState::Playing { song_id, mut art_id, paused } => {
                if y_val < 1000 {
                    if art_id == 0 { art_id = max_arts - 1; } else { art_id -= 1; }
                    Timer::after(Duration::from_millis(300)).await;
                } else if y_val > 3000 {
                    if art_id == max_arts - 1 { art_id = 0; } else { art_id += 1; }
                    Timer::after(Duration::from_millis(300)).await;
                }

                if b_just_pressed {
                    AppState::Playing { song_id, art_id, paused: !paused }
                } else if a_just_pressed {
                    AppState::Menu { song_id, art_id }
                } else {
                    AppState::Playing { song_id, art_id, paused }
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