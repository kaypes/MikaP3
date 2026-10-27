//! Deterministic application controller. Time and entropy come from the caller.

use crate::arts::ARTS;
use crate::audio::AudioRequest;
use crate::controls::{AxisDirection, ControlContext, ControlFrame, Controls};
use crate::flappy::game::FlappyGame;
use crate::snake::game::{Direction, SnakeGame};
use crate::songs::{HINO_NACIONAL, SONGS, TETRIS};
use crate::state::{AppState, GameId, Screen};

pub use crate::controls::{INPUT_INTERVAL_MS, InputSample};

#[cfg(test)]
mod tests;

pub const APP_TICK_INTERVAL_MS: u64 = 50;
pub const ANIMATION_DURATION_MS: u64 = 1500;

#[derive(Debug, Clone, Copy)]
pub struct Time {
    pub millis: u64,
    pub seed: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    Input(InputSample),
    Tick,
    AudioFinished { session: u64 },
    AudioFailed { session: u64 },
}

#[derive(Default)]
pub struct Effects {
    pub reset_usb: bool,
}

struct Animation {
    return_state: AppState,
    started: u64,
}

pub struct Controller {
    state: AppState,
    controls: Controls,
    audio: AudioRequest,
    audio_source: Option<(Screen, u8)>,
    next_session: u64,
    last_game_move: u64,
    music_selection: (u8, u8),
    animation: Option<Animation>,
}

impl Controller {
    pub fn new(now: Time) -> Self {
        Self {
            state: AppState::INITIAL,
            controls: Controls::default(),
            audio: AudioRequest::Silence,
            audio_source: None,
            next_session: 0,
            last_game_move: now.millis,
            music_selection: (0, 0),
            animation: None,
        }
    }

    pub fn state(&self) -> AppState {
        self.state
    }
    pub fn audio_request(&self) -> AudioRequest {
        self.audio
    }

    pub fn handle(&mut self, event: AppEvent, now: Time) -> Effects {
        let previous_context = interaction(self.state);
        let previous_art = selected_art(self.effective_state());
        let mut effects = Effects::default();
        if self.animation.as_ref().is_some_and(|animation| {
            now.millis.saturating_sub(animation.started) >= ANIMATION_DURATION_MS
        }) {
            self.state = self.animation.take().unwrap().return_state;
            self.controls.rearm();
        }

        match event {
            AppEvent::Input(sample) => {
                let context = ControlContext {
                    triple_joy: selected_art(self.state) == Some(0),
                    allow_ab: matches!(self.state, AppState::FlappyBird(game, _) if !game.game_over),
                    suppress_actions: self.animation.is_some(),
                };
                let frame = self.controls.sample(sample, now.millis, context);
                effects.reset_usb = frame.reset_usb;
                if !frame.reset_usb && self.animation.is_none() {
                    self.input(frame, now);
                }
            }
            AppEvent::Tick => {
                if self.animation.is_none() {
                    self.advance_game(now);
                }
            }
            AppEvent::AudioFinished { session } | AppEvent::AudioFailed { session }
                if self.audio.session() == Some(session) =>
            {
                let next = match self.effective_state() {
                    AppState::Playing {
                        song_id, art_id, ..
                    } => AppState::Menu { song_id, art_id },
                    AppState::Snake(game, _) => AppState::Snake(game, false),
                    AppState::FlappyBird(game, _) => AppState::FlappyBird(game, false),
                    state => state,
                };
                if let Some(animation) = &mut self.animation {
                    animation.return_state = next;
                } else {
                    self.state = next;
                }
            }
            AppEvent::AudioFinished { .. } | AppEvent::AudioFailed { .. } => {}
        }

        if previous_context != interaction(self.state) {
            self.controls.rearm();
        }
        if previous_art != selected_art(self.effective_state()) {
            self.controls.cancel_triple();
        }
        if let AppState::Menu { song_id, art_id }
        | AppState::Playing {
            song_id, art_id, ..
        } = self.effective_state()
        {
            self.music_selection = (song_id, art_id);
        }
        self.update_audio();
        effects
    }

    fn effective_state(&self) -> AppState {
        self.animation
            .as_ref()
            .map_or(self.state, |animation| animation.return_state)
    }

    fn music_menu(&self) -> AppState {
        AppState::Menu {
            song_id: self.music_selection.0,
            art_id: self.music_selection.1,
        }
    }

    fn input(&mut self, frame: ControlFrame, now: Time) {
        if frame.joy_long {
            self.state = match self.state {
                AppState::Menu { .. } | AppState::Playing { .. } => AppState::GamesMenu {
                    game_id: GameId::Snake,
                    with_music: true,
                },
                AppState::GamesMenu { .. } => self.music_menu(),
                AppState::Snake(_, with_music) => AppState::GamesMenu {
                    game_id: GameId::Snake,
                    with_music,
                },
                AppState::FlappyBird(_, with_music) => AppState::GamesMenu {
                    game_id: GameId::FlappyBird,
                    with_music,
                },
                state => state,
            };
            return;
        }
        if frame.joy_triple && selected_art(self.state) == Some(0) {
            self.animation = Some(Animation {
                return_state: self.state,
                started: now.millis,
            });
            self.state = AppState::EasterEgg;
            return;
        }

        self.state = match self.state {
            AppState::Menu { song_id, art_id } => {
                let song_id = cycle(song_id, SONGS.len(), frame.x_step);
                let art_id = cycle(art_id, ARTS.len(), frame.y_step);
                if frame.b_pressed {
                    AppState::Playing {
                        song_id,
                        art_id,
                        paused: false,
                    }
                } else {
                    AppState::Menu { song_id, art_id }
                }
            }
            AppState::Playing {
                song_id,
                art_id,
                paused,
            } => {
                let art_id = cycle(art_id, ARTS.len(), frame.y_step);
                if frame.a_pressed {
                    AppState::Menu { song_id, art_id }
                } else {
                    AppState::Playing {
                        song_id,
                        art_id,
                        paused: paused ^ frame.b_pressed,
                    }
                }
            }
            AppState::GamesMenu {
                mut game_id,
                with_music,
            } => {
                if let Some(direction) = frame.x_step {
                    game_id = game_id.cycle(direction == AxisDirection::High);
                }
                if frame.a_pressed {
                    self.music_menu()
                } else if frame.b_pressed {
                    self.last_game_move = now.millis;
                    match game_id {
                        GameId::Snake => AppState::Snake(SnakeGame::new(now.seed), with_music),
                        GameId::FlappyBird => {
                            AppState::FlappyBird(FlappyGame::new(now.seed), with_music)
                        }
                    }
                } else {
                    AppState::GamesMenu {
                        game_id,
                        with_music: with_music ^ frame.joy_clicked,
                    }
                }
            }
            AppState::Snake(mut game, with_music) => {
                if frame.a_pressed {
                    AppState::GamesMenu {
                        game_id: GameId::Snake,
                        with_music,
                    }
                } else if game.game_over && frame.b_pressed {
                    self.last_game_move = now.millis;
                    AppState::Snake(SnakeGame::new(now.seed), with_music)
                } else {
                    if !game.game_over {
                        let was_idle = game.dir == Direction::Idle;
                        game.input(snake_direction(frame.x, frame.y, game.dir));
                        if was_idle && game.dir != Direction::Idle {
                            game.step(now.seed);
                            self.last_game_move = now.millis;
                        }
                    }
                    AppState::Snake(game, with_music)
                }
            }
            AppState::FlappyBird(mut game, with_music) => {
                if game.game_over && frame.a_pressed {
                    AppState::GamesMenu {
                        game_id: GameId::FlappyBird,
                        with_music,
                    }
                } else if game.game_over && frame.b_pressed {
                    self.last_game_move = now.millis;
                    AppState::FlappyBird(FlappyGame::new(now.seed), with_music)
                } else {
                    if frame.a_pressed || frame.b_pressed {
                        game.jump();
                    }
                    AppState::FlappyBird(game, with_music)
                }
            }
            AppState::EasterEgg => self.state,
        };
    }

    fn advance_game(&mut self, now: Time) {
        match &mut self.state {
            AppState::Snake(game, _)
                if !game.game_over
                    && game.dir != Direction::Idle
                    && now.millis.saturating_sub(self.last_game_move)
                        > u64::from(game.speed_ms) =>
            {
                game.step(now.seed);
                self.last_game_move = now.millis;
            }
            AppState::FlappyBird(game, _)
                if !game.game_over
                    && now.millis.saturating_sub(self.last_game_move)
                        > u64::from(game.speed_ms) =>
            {
                game.step(now.seed);
                self.last_game_move = now.millis;
            }
            _ => {}
        }
    }

    fn update_audio(&mut self) {
        let state = self.effective_state();
        let source = match state {
            AppState::Playing { song_id, .. } => Some((Screen::Playing, song_id)),
            AppState::Snake(_, true) => Some((Screen::Snake, HINO_NACIONAL)),
            AppState::FlappyBird(_, true) => Some((Screen::FlappyBird, TETRIS)),
            _ => None,
        };
        if source != self.audio_source {
            self.next_session = self.next_session.wrapping_add(1);
            self.audio_source = source;
        }
        self.audio = if let Some((screen, song_id)) = source {
            AudioRequest::Play {
                session: self.next_session,
                song_id,
                repeat: screen != Screen::Playing,
                paused: matches!(state, AppState::Playing { paused: true, .. }),
            }
        } else {
            AudioRequest::Silence
        };
    }
}

fn selected_art(state: AppState) -> Option<u8> {
    match state {
        AppState::Menu { art_id, .. } | AppState::Playing { art_id, .. } => Some(art_id),
        _ => None,
    }
}

fn interaction(state: AppState) -> (Screen, bool) {
    let game_over = match state {
        AppState::Snake(game, _) => game.game_over,
        AppState::FlappyBird(game, _) => game.game_over,
        _ => false,
    };
    (state.screen(), game_over)
}

fn cycle(id: u8, count: usize, step: Option<AxisDirection>) -> u8 {
    match step {
        Some(AxisDirection::Low) => ((usize::from(id) + count - 1) % count) as u8,
        Some(AxisDirection::High) => ((usize::from(id) + 1) % count) as u8,
        _ => id,
    }
}

fn snake_direction(x: AxisDirection, y: AxisDirection, current: Direction) -> Direction {
    // Preserve board orientation and Y-over-X diagonal priority.
    match (x, y) {
        (_, AxisDirection::High) => Direction::Right,
        (_, AxisDirection::Low) => Direction::Left,
        (AxisDirection::High, _) => Direction::Up,
        (AxisDirection::Low, _) => Direction::Down,
        _ => current,
    }
}
