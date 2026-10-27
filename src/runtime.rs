//! Embassy wiring. Only application_task can publish application/audio state.

use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::watch::{Receiver, Watch};
use embassy_time::{Duration, Instant, Ticker};
use mikap3::app::{AppEvent, Controller, INPUT_INTERVAL_MS, Time};
use mikap3::audio::AudioRequest;
use mikap3::state::AppState;

static EVENTS: Channel<CriticalSectionRawMutex, AppEvent, 16> = Channel::new();
static STATE: Watch<CriticalSectionRawMutex, AppState, 2> = Watch::new_with(AppState::INITIAL);
static AUDIO: Watch<CriticalSectionRawMutex, AudioRequest, 1> =
    Watch::new_with(AudioRequest::Silence);

pub type StateReceiver = Receiver<'static, CriticalSectionRawMutex, AppState, 2>;
pub type AudioReceiver = Receiver<'static, CriticalSectionRawMutex, AudioRequest, 1>;

pub fn subscribe_state() -> StateReceiver {
    STATE.receiver().unwrap()
}
pub fn subscribe_audio() -> AudioReceiver {
    AUDIO.receiver().unwrap()
}

pub async fn send_event(event: AppEvent) {
    EVENTS.send(event).await;
}

fn now() -> Time {
    let instant = Instant::now();
    Time {
        millis: instant.as_millis(),
        seed: instant.as_ticks(),
    }
}

#[embassy_executor::task]
pub async fn application_task() {
    let mut controller = Controller::new(now());
    let mut ticker = Ticker::every(Duration::from_millis(INPUT_INTERVAL_MS));
    loop {
        let event = match select(EVENTS.receive(), ticker.next()).await {
            Either::First(event) => event,
            Either::Second(_) => AppEvent::Tick,
        };
        let previous_state = controller.state();
        let previous_audio = controller.audio_request();
        let effects = controller.handle(event, now());
        if previous_state != controller.state() {
            STATE.sender().send(controller.state());
        }
        if previous_audio != controller.audio_request() {
            AUDIO.sender().send(controller.audio_request());
        }
        if effects.reset_usb {
            embassy_rp::rom_data::reset_to_usb_boot(0, 0);
        }
    }
}
