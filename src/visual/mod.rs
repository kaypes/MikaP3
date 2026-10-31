//! Logical RGB frames and OLED content; no board wiring or peripheral access.

use crate::arts::{
    ARTS, BROCOS_ICON, CORACAO_ANIMATION, FLAPPY_ICON, SNAKE_ICON, sprites::Sprite,
};
use crate::brocos::game::{BrocosGame, CELEBRATION_MS, Status};
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

/// Only changes that can alter the OLED pixels wake `display_task`.
pub fn oled_relevant_change(previous: AppState, current: AppState) -> bool {
    match (previous, current) {
        (AppState::Brocos(a, _), AppState::Brocos(b, _)) => {
            (
                a.score,
                a.lives,
                a.level,
                a.status,
                a.bricks(),
                a.paddle_x(),
                a.ball_position(),
            ) != (
                b.score,
                b.lives,
                b.level,
                b.status,
                b.bricks(),
                b.paddle_x(),
                b.ball_position(),
            )
        }
        _ => DisplayView::from(previous) != DisplayView::from(current),
    }
}

/// Only changes that can alter the 5x5 matrix frame or its animation schedule wake `leds_task`.
pub fn leds_relevant_change(previous: AppState, current: AppState) -> bool {
    match (previous, current) {
        (
            AppState::Menu { art_id: a, .. } | AppState::Playing { art_id: a, .. },
            AppState::Menu { art_id: b, .. } | AppState::Playing { art_id: b, .. },
        ) => a != b,
        (AppState::Brocos(a, _), AppState::Brocos(b, _)) => {
            let a_celebrating = matches!(a.status, Status::Celebrating { .. });
            let b_celebrating = matches!(b.status, Status::Celebrating { .. });
            (a_celebrating || b_celebrating) && a.status != b.status
        }
        (AppState::GamesMenu { game_id: a, .. }, AppState::GamesMenu { game_id: b, .. }) => a != b,
        (AppState::Snake(a, _), AppState::Snake(b, _)) => {
            (a.head, a.fruit, a.obstacles, a.game_over)
                != (b.head, b.fruit, b.obstacles, b.game_over)
        }
        (AppState::FlappyBird(a, _), AppState::FlappyBird(b, _)) => {
            (a.bird_y, a.pipe_x, a.pipe_gap, a.game_over)
                != (b.bird_y, b.pipe_x, b.pipe_gap, b.game_over)
        }
        _ => previous != current,
    }
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
            AppState::Brocos(game, _) => {
                let mut frame = [(0, 0, 0); 25];
                if let Status::Celebrating { started_ms } = game.status {
                    let elapsed = now_ms.saturating_sub(started_ms);
                    if elapsed < CELEBRATION_MS {
                        let index = elapsed / 100;
                        let colors = [(10, 0, 0), (10, 5, 0), (0, 5, 0), (0, 5, 3), (0, 0, 3)];
                        for (pixel, color) in frame.iter_mut().enumerate() {
                            *color = colors[(pixel + index as usize) % colors.len()];
                        }
                        next_wake_ms = Some(started_ms + (index + 1) * 100);
                    }
                }
                frame
            }
            AppState::GamesMenu { game_id, .. } => sprite_frame(match game_id {
                GameId::Snake => &SNAKE_ICON,
                GameId::FlappyBird => &FLAPPY_ICON,
                GameId::Brocos => &BROCOS_ICON,
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
    Brocos(BrocosGame),
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

impl DisplayView {
    pub const INITIAL: Self = Self::Menu {
        song: SONGS[0].name,
        art: ARTS[0].name,
    };

    /// Two rows in FONT_6X10, each within the 128-pixel OLED width.
    pub fn control_hints(self) -> [&'static str; 2] {
        match self {
            Self::Brocos(_)
            | Self::Score {
                game: GameId::Brocos,
                ..
            } => ["", ""],
            Self::Menu { .. } => ["V:Tocar", "J2s:Jogos"],
            Self::Playing { .. } => ["A:Voltar V:Pausa", "J2s:Jogos"],
            Self::GamesMenu { .. } => ["A:Voltar V:Jogar", "J:Som J2s:Musicas"],
            Self::Score {
                game_over: true, ..
            } => ["A:Voltar V:Reiniciar", "J2s:Voltar"],
            Self::Score {
                game: GameId::FlappyBird,
                ..
            } => ["A/V:Salto", "J2s:Voltar"],
            Self::Score {
                game: GameId::Snake,
                ..
            } => ["Joystick:Direcao", "A:Voltar J2s:Voltar"],
            Self::EasterEgg => ["A+V+J 2s:USB", ""],
        }
    }
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
            AppState::Brocos(game, _) => Self::Brocos(game),
            AppState::EasterEgg => Self::EasterEgg,
        }
    }
}

/// Limit OLED playfield redraws without delaying transitions to other screens.
#[derive(Default)]
pub struct OledCadence {
    last_brocos_draw: Option<u64>,
}

impl OledCadence {
    pub fn wait_until(&self, view: DisplayView, now_ms: u64) -> Option<u64> {
        if matches!(view, DisplayView::Brocos(_)) {
            self.last_brocos_draw
                .map(|last| last + 50)
                .filter(|&deadline| now_ms < deadline)
        } else {
            None
        }
    }

    pub fn drawn(&mut self, view: DisplayView, now_ms: u64) {
        self.last_brocos_draw = matches!(view, DisplayView::Brocos(_)).then_some(now_ms);
    }
}
