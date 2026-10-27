pub mod always_with_me;
pub mod fallen_down;
pub mod game_of_thrones;
pub mod hino_nacional;
pub mod his_theme;
pub mod le_festin;
pub mod love_like_you;
pub mod married_life;
pub mod minuet_in_g_major;
pub mod new_horizons;
pub mod tetris;

use crate::audio::Note;

pub struct Song {
    pub name: &'static str,
    pub track_a: &'static [Note],
    pub track_b: &'static [Note],
}

pub const HINO_NACIONAL: u8 = 4;
pub const TETRIS: u8 = 10;

pub static SONGS: &[Song] = &[
    Song {
        name: "Married Life",
        track_a: married_life::TRACK_A,
        track_b: married_life::TRACK_B,
    },
    Song {
        name: "Always with Me",
        track_a: always_with_me::TRACK_A,
        track_b: always_with_me::TRACK_B,
    },
    Song {
        name: "Fallen Down",
        track_a: fallen_down::TRACK_A,
        track_b: fallen_down::TRACK_B,
    },
    Song {
        name: "Game of Thrones",
        track_a: game_of_thrones::TRACK_A,
        track_b: game_of_thrones::TRACK_B,
    },
    Song {
        name: "Hino Nacional",
        track_a: hino_nacional::TRACK_A,
        track_b: hino_nacional::TRACK_B,
    },
    Song {
        name: "His Theme",
        track_a: his_theme::TRACK_A,
        track_b: his_theme::TRACK_B,
    },
    Song {
        name: "Le Festin",
        track_a: le_festin::TRACK_A,
        track_b: le_festin::TRACK_B,
    },
    Song {
        name: "Love Like You",
        track_a: love_like_you::TRACK_A,
        track_b: love_like_you::TRACK_B,
    },
    Song {
        name: "Minuet in G",
        track_a: minuet_in_g_major::TRACK_A,
        track_b: minuet_in_g_major::TRACK_B,
    },
    Song {
        name: "New Horizons",
        track_a: new_horizons::TRACK_A,
        track_b: new_horizons::TRACK_B,
    },
    Song {
        name: "Tetris",
        track_a: tetris::TRACK_A,
        track_b: tetris::TRACK_B,
    },
];
