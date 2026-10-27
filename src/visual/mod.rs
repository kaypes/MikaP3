//! Logical RGB frames and OLED content; no board wiring or peripheral access.

use crate::arts::{ARTS, CORACAO_ANIMATION, FLAPPY_ICON, SNAKE_ICON, sprites::Sprite};
use crate::songs::SONGS;
use crate::state::{AppState, GameId};

#[cfg(test)]
mod tests;

pub type Frame = [(u8, u8, u8); 25];
pub const ANIMATION_FRAME_MS: u64 = 150;

pub fn sprite_frame(sprite: &Sprite) -> Frame {
    let mut frame = [(0, 0, 0); 25];
    for (y, row) in sprite.iter().enumerate() {
        for (x, symbol) in row.bytes().take(5).enumerate() {
            frame[y * 5 + x] = match symbol {
                b'R' => (10, 0, 0),
                b'G' => (0, 5, 0),
                b'B' => (0, 0, 3),
                b'Y' => (10, 5, 0),
                b'P' => (10, 0, 3),
                b'C' => (0, 5, 3),
                b'W' => (10, 5, 3),
                b'O' => (10, 2, 0),
                b'L' => (5, 3, 0),
                _ => (0, 0, 0),
            };
        }
    }
    frame
}

pub struct MatrixUpdate {
    pub frame: Option<Frame>,
    pub next_wake_ms: Option<u64>,
}

#[derive(Default)]
pub struct MatrixRenderer {
    last_frame: Option<Frame>,
    animation_started: Option<u64>,
}

impl MatrixRenderer {
    pub fn update(&mut self, state: AppState, now_ms: u64) -> MatrixUpdate {
        let mut next_wake_ms = None;
        if !matches!(state, AppState::EasterEgg) {
            self.animation_started = None;
        }
        let frame = match state {
            AppState::Menu { art_id, .. } | AppState::Playing { art_id, .. } => {
                sprite_frame(&ARTS[usize::from(art_id)].sprite)
            }
            AppState::Snake(game, _) => crate::snake::render::draw_frame(&game),
            AppState::FlappyBird(game, _) => crate::flappy::render::draw_frame(&game),
            AppState::GamesMenu { game_id, .. } => sprite_frame(match game_id {
                GameId::Snake => &SNAKE_ICON,
                GameId::FlappyBird => &FLAPPY_ICON,
            }),
            AppState::EasterEgg => {
                let start = *self.animation_started.get_or_insert(now_ms);
                let index = (now_ms.saturating_sub(start) / ANIMATION_FRAME_MS)
                    .min(CORACAO_ANIMATION.len() as u64 - 1) as usize;
                if index + 1 < CORACAO_ANIMATION.len() {
                    next_wake_ms = Some(start + (index as u64 + 1) * ANIMATION_FRAME_MS);
                }
                sprite_frame(&CORACAO_ANIMATION[index])
            }
        };
        let changed = self.last_frame != Some(frame);
        self.last_frame = Some(frame);
        MatrixUpdate {
            frame: changed.then_some(frame),
            next_wake_ms,
        }
    }
}

/// Only fields visible on the OLED participate in its redraw decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayView {
    Menu {
        song: &'static str,
        art: &'static str,
    },
    Playing {
        song: &'static str,
        art: &'static str,
        paused: bool,
    },
    GamesMenu {
        game: GameId,
        with_music: bool,
    },
    Score {
        game: GameId,
        score: u32,
        game_over: bool,
    },
    EasterEgg,
}

impl From<AppState> for DisplayView {
    fn from(state: AppState) -> Self {
        match state {
            AppState::Menu { song_id, art_id } => Self::Menu {
                song: SONGS[usize::from(song_id)].name,
                art: ARTS[usize::from(art_id)].name,
            },
            AppState::Playing {
                song_id,
                art_id,
                paused,
            } => Self::Playing {
                song: SONGS[usize::from(song_id)].name,
                art: ARTS[usize::from(art_id)].name,
                paused,
            },
            AppState::GamesMenu {
                game_id,
                with_music,
            } => Self::GamesMenu {
                game: game_id,
                with_music,
            },
            AppState::Snake(game, _) => Self::Score {
                game: GameId::Snake,
                score: game.score,
                game_over: game.game_over,
            },
            AppState::FlappyBird(game, _) => Self::Score {
                game: GameId::FlappyBird,
                score: game.score,
                game_over: game.game_over,
            },
            AppState::EasterEgg => Self::EasterEgg,
        }
    }
}
