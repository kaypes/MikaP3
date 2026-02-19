use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::Watch;

#[derive(Clone, Copy, PartialEq)]
pub enum AppState {
    Menu {
        song_id: u8,
        art_id: u8,
    },
    Playing {
        song_id: u8,
        art_id: u8,
        paused: bool,
    },
}

pub static STATE: Watch<CriticalSectionRawMutex, AppState, 4> = Watch::new_with(AppState::Menu {
    song_id: 0,
    art_id: 0,
});
