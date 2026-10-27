use embassy_futures::select::select;
use embassy_rp::pwm::Pwm;
use embassy_time::Timer;
use mikap3::app::AppEvent;
use mikap3::audio::AudioRequest;
use mikap3::songs::SONGS;

use crate::buzzer::{self, Session};
use crate::runtime::{self, AudioReceiver};

#[embassy_executor::task]
pub async fn audio_task(mut pwm_a: Pwm<'static>, mut pwm_b: Pwm<'static>) {
    let mut receiver = runtime::subscribe_audio();
    loop {
        match receiver.get().await {
            AudioRequest::Silence => {
                buzzer::mute(&mut pwm_a, &mut pwm_b);
                receiver.changed().await;
            }
            AudioRequest::Play {
                session,
                song_id,
                repeat,
                ..
            } => {
                if let Some(event) = play_session(
                    &mut pwm_a,
                    &mut pwm_b,
                    &mut receiver,
                    session,
                    song_id,
                    repeat,
                )
                .await
                {
                    // The output guard has already been dropped before waiting for channel space.
                    runtime::send_event(event).await;
                    // A queued pause/resume is still the completed session. Do not
                    // restart it while the controller processes the terminal event.
                    while receiver.get().await.session() == Some(session) {
                        receiver.changed().await;
                    }
                }
            }
        }
    }
}

async fn play_session<'d>(
    pwm_a: &mut Pwm<'d>,
    pwm_b: &mut Pwm<'d>,
    receiver: &mut AudioReceiver,
    id: u64,
    song_id: u8,
    repeat: bool,
) -> Option<AppEvent> {
    let failure = Some(AppEvent::AudioFailed { session: id });
    let Some(song) = SONGS.get(usize::from(song_id)) else {
        return failure;
    };
    let Some(mut session) = Session::new(pwm_a, pwm_b, song.track_a, song.track_b, repeat) else {
        return failure;
    };
    loop {
        let paused = match receiver.get().await {
            AudioRequest::Play {
                session, paused, ..
            } if session == id => paused,
            _ => return None,
        };
        session.set_paused(paused);
        let (finished, deadline) = session.update();
        if finished {
            return if repeat {
                failure
            } else {
                Some(AppEvent::AudioFinished { session: id })
            };
        }
        if let Some(deadline) = deadline {
            // A new command takes priority over a simultaneous note deadline.
            select(receiver.changed(), Timer::at(deadline)).await;
        } else {
            receiver.changed().await;
        }
    }
}
