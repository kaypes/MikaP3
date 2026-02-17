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
    mut btn_a: Input<'static>,
    mut btn_b: Input<'static>,
) {
    let max_songs: u8 = 7;
    let max_arts: u8 = 7;

    let mut rx = STATE.receiver().unwrap();
    let mut current_state: AppState = rx.get().await;

    loop {
        if let Either::First(new_state) = select(rx.changed(), Timer::after(Duration::from_ticks(0))).await {
            current_state = new_state;
        }

        let mut next_state = current_state;

        let x_val = adc.read(&mut joy_x).await.unwrap_or(2048);
        let y_val = adc.read(&mut joy_y).await.unwrap_or(2048);
        
        let a_pressed = btn_a.is_low();
        let b_pressed = btn_b.is_low();

        match current_state {
            AppState::Menu { mut song_id, mut art_id } => {
                if x_val < 1000 && song_id > 0 {
                    song_id -= 1;
                    Timer::after(Duration::from_millis(300)).await;
                } else if x_val > 3000 && song_id < max_songs - 1 {
                    song_id += 1;
                    Timer::after(Duration::from_millis(300)).await;
                }

                if y_val < 1000 && art_id > 0 {
                    art_id -= 1;
                    Timer::after(Duration::from_millis(300)).await;
                } else if y_val > 3000 && art_id < max_arts - 1 {
                    art_id += 1;
                    Timer::after(Duration::from_millis(300)).await;
                }

                if a_pressed {
                    next_state = AppState::Playing { song_id, art_id, paused: false };
                    Timer::after(Duration::from_millis(300)).await;
                } else {
                    next_state = AppState::Menu { song_id, art_id };
                }
            }
            AppState::Playing { song_id, art_id, mut paused } => {
                if a_pressed {
                    paused = !paused;
                    next_state = AppState::Playing { song_id, art_id, paused };
                    Timer::after(Duration::from_millis(300)).await;
                }
                if b_pressed {
                    next_state = AppState::Menu { song_id, art_id };
                    Timer::after(Duration::from_millis(300)).await;
                }
            }
        }

        if next_state != current_state {
            STATE.sender().send(next_state);
            current_state = next_state;
        }

        Timer::after(Duration::from_millis(50)).await;
    }
}