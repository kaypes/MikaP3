use embassy_rp::adc::{Adc, Async, Channel as AdcChannel};
use embassy_rp::gpio::Input;
use embassy_time::{Duration, Instant, Ticker};
use mikap3::app::AppEvent;
use mikap3::controls::{IDLE_SETTLE_MS, INPUT_INTERVAL_MS, InputSample};

#[embassy_executor::task]
pub async fn input_task(
    mut adc: Adc<'static, Async>,
    mut joy_x: AdcChannel<'static>,
    mut joy_y: AdcChannel<'static>,
    btn_a: Input<'static>,
    btn_b: Input<'static>,
    btn_joy: Input<'static>,
) {
    let mut ticker = Ticker::every(Duration::from_millis(INPUT_INTERVAL_MS));
    let mut idle_since: Option<u64> = None;
    loop {
        let sample = InputSample {
            a: btn_a.is_low(),
            b: btn_b.is_low(),
            joy: btn_joy.is_low(),
            x: adc.read(&mut joy_x).await.unwrap_or(2048),
            y: adc.read(&mut joy_y).await.unwrap_or(2048),
        };
        let now = Instant::now().as_millis();
        let should_send = if sample.is_resting() {
            let start = *idle_since.get_or_insert(now);
            now.saturating_sub(start) <= IDLE_SETTLE_MS
        } else {
            idle_since = None;
            true
        };
        if should_send {
            crate::runtime::send_event(AppEvent::Input(sample)).await;
        }
        ticker.next().await;
    }
}
