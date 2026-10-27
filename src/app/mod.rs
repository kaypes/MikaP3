//! Deterministic application controller. Time and entropy come from the caller.

use crate::arts::ARTS;
use crate::audio::AudioRequest;
use crate::flappy::game::FlappyGame;
use crate::snake::game::{Direction, SnakeGame};
use crate::songs::{HINO_NACIONAL, SONGS, TETRIS};
use crate::state::{AppState, GameId, Screen};

#[cfg(test)]
mod tests;

pub const INPUT_INTERVAL_MS: u64 = 50;

#[derive(Debug, Clone, Copy)]
pub struct InputSample {
    pub a: bool,
    pub b: bool,
    pub joy: bool,
    pub x: u16,
    pub y: u16,
}

impl Default for InputSample {
    fn default() -> Self {
        Self {
            a: false,
            b: false,
            joy: false,
            x: 2048,
            y: 2048,
        }
    }
}

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

pub struct Controller {
    state: AppState,
    audio: AudioRequest,
    audio_source: Option<(Screen, u8)>,
    next_session: u64,
    a_was_pressed: bool,
    b_was_pressed: bool,
    last_joy_move: u64,
    last_game_move: u64,
    a_hold_start: Option<u64>,
    a_click_count: u8,
    last_a_click: u64,
    easter_egg_start: Option<u64>,
    reset_hold_start: Option<u64>,
}

impl Controller {
    pub fn new(now: Time) -> Self {
        Self {
            state: AppState::INITIAL,
            audio: AudioRequest::Silence,
            audio_source: None,
            next_session: 0,
            a_was_pressed: false,
            b_was_pressed: false,
            last_joy_move: now.millis,
            last_game_move: now.millis,
            a_hold_start: None,
            a_click_count: 0,
            last_a_click: now.millis,
            easter_egg_start: None,
            reset_hold_start: None,
        }
    }

    pub fn state(&self) -> AppState {
        self.state
    }

    pub fn audio_request(&self) -> AudioRequest {
        self.audio
    }

    pub fn handle(&mut self, event: AppEvent, now: Time) -> Effects {
        let mut effects = Effects::default();
        match event {
            AppEvent::Input(sample) => self.input(sample, now, &mut effects),
            AppEvent::Tick => {
                if !self.expire_easter_egg(now.millis) {
                    self.advance_game(now);
                }
            }
            AppEvent::AudioFinished { session } | AppEvent::AudioFailed { session }
                if self.audio.session() == Some(session) =>
            {
                self.state = match self.state {
                    AppState::Playing {
                        song_id, art_id, ..
                    } => AppState::Menu { song_id, art_id },
                    // Failed/empty looping tracks stay silent without retrying every tick.
                    AppState::Snake(game, _) => AppState::Snake(game, false),
                    AppState::FlappyBird(game, _) => AppState::FlappyBird(game, false),
                    state => state,
                };
            }
            AppEvent::AudioFinished { .. } | AppEvent::AudioFailed { .. } => {}
        }
        self.update_audio();
        effects
    }

    fn expire_easter_egg(&mut self, now: u64) -> bool {
        if let Some(start) = self.easter_egg_start {
            if now.saturating_sub(start) > 1500 {
                self.easter_egg_start = None;
                self.state = AppState::INITIAL;
            }
            true
        } else {
            false
        }
    }

    fn input(&mut self, sample: InputSample, now: Time, effects: &mut Effects) {
        let ms = now.millis;
        // Preserve the original gesture ordering, including ignoring input in the animation.
        if self.expire_easter_egg(ms) {
            return;
        }

        if sample.a && sample.b && sample.joy {
            let start = *self.reset_hold_start.get_or_insert(ms);
            effects.reset_usb = ms.saturating_sub(start) >= 2000;
        } else {
            self.reset_hold_start = None;
        }

        let mut override_state = None;
        if sample.a && !sample.b && !sample.joy {
            let start = *self.a_hold_start.get_or_insert(ms);
            if ms.saturating_sub(start) >= 2000 {
                override_state = Some(match self.state {
                    AppState::Menu { .. } | AppState::Playing { .. } => AppState::GamesMenu {
                        game_id: GameId::Snake,
                        with_music: true,
                    },
                    AppState::GamesMenu { .. } => AppState::INITIAL,
                    AppState::Snake(_, with_music) => AppState::GamesMenu {
                        game_id: GameId::Snake,
                        with_music,
                    },
                    AppState::FlappyBird(_, with_music) => AppState::GamesMenu {
                        game_id: GameId::FlappyBird,
                        with_music,
                    },
                    state => state,
                });
                self.a_hold_start = None;
            }
        } else {
            self.a_hold_start = None;
        }

        let a_pressed = sample.a && !self.a_was_pressed;
        let b_pressed = sample.b && !self.b_was_pressed;
        self.a_was_pressed = sample.a;
        self.b_was_pressed = sample.b;

        if a_pressed {
            self.a_click_count = if ms.saturating_sub(self.last_a_click) < 500 {
                self.a_click_count.saturating_add(1)
            } else {
                1
            };
            self.last_a_click = ms;
        } else if !sample.a && ms.saturating_sub(self.last_a_click) > 500 {
            self.a_click_count = 0;
        }

        if self.a_click_count == 3
            && matches!(
                self.state,
                AppState::Menu { art_id: 0, .. } | AppState::Playing { art_id: 0, .. }
            )
        {
            self.a_click_count = 0;
            self.easter_egg_start = Some(ms);
            self.state = AppState::EasterEgg;
            return;
        }

        let joy_ready = ms.saturating_sub(self.last_joy_move) > 300;
        if let Some(state) = override_state {
            self.state = state;
            return;
        }

        self.state = match self.state {
            AppState::Menu { song_id, art_id } => {
                let (song_id, song_moved) = carousel(song_id, SONGS.len(), sample.x, joy_ready);
                let (art_id, art_moved) = carousel(art_id, ARTS.len(), sample.y, joy_ready);
                if song_moved || art_moved {
                    self.last_joy_move = ms;
                }
                if b_pressed {
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
                let (art_id, moved) = carousel(art_id, ARTS.len(), sample.y, joy_ready);
                if moved {
                    self.last_joy_move = ms;
                }
                match (b_pressed, a_pressed) {
                    (true, _) => AppState::Playing {
                        song_id,
                        art_id,
                        paused: !paused,
                    },
                    (false, true) => AppState::Menu { song_id, art_id },
                    _ => AppState::Playing {
                        song_id,
                        art_id,
                        paused,
                    },
                }
            }
            AppState::GamesMenu {
                mut game_id,
                with_music,
            } => {
                if joy_ready && (sample.x <= 1000 || sample.x >= 3000) {
                    game_id = game_id.cycle(sample.x >= 3000);
                    self.last_joy_move = ms;
                }
                if b_pressed {
                    self.last_game_move = ms;
                    match game_id {
                        GameId::Snake => AppState::Snake(SnakeGame::new(now.seed), with_music),
                        GameId::FlappyBird => {
                            AppState::FlappyBird(FlappyGame::new(now.seed), with_music)
                        }
                    }
                } else {
                    AppState::GamesMenu {
                        game_id,
                        with_music: with_music ^ a_pressed,
                    }
                }
            }
            AppState::Snake(mut game, with_music) => {
                if game.game_over {
                    if a_pressed {
                        AppState::GamesMenu {
                            game_id: GameId::Snake,
                            with_music,
                        }
                    } else if b_pressed {
                        self.last_game_move = ms;
                        AppState::Snake(SnakeGame::new(now.seed), with_music)
                    } else {
                        AppState::Snake(game, with_music)
                    }
                } else {
                    let was_idle = game.dir == Direction::Idle;
                    game.input(snake_direction(sample, game.dir));
                    if was_idle && game.dir != Direction::Idle {
                        game.step(now.seed);
                        self.last_game_move = ms;
                    }
                    AppState::Snake(game, with_music)
                }
            }
            AppState::FlappyBird(mut game, with_music) => {
                if game.game_over {
                    if a_pressed {
                        AppState::GamesMenu {
                            game_id: GameId::FlappyBird,
                            with_music,
                        }
                    } else if b_pressed {
                        self.last_game_move = ms;
                        AppState::FlappyBird(FlappyGame::new(now.seed), with_music)
                    } else {
                        AppState::FlappyBird(game, with_music)
                    }
                } else {
                    if a_pressed || b_pressed || sample.joy {
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
        let source = match self.state {
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
                paused: matches!(self.state, AppState::Playing { paused: true, .. }),
            }
        } else {
            AudioRequest::Silence
        };
    }
}

fn carousel(id: u8, count: usize, value: u16, enabled: bool) -> (u8, bool) {
    if !enabled {
        return (id, false);
    }
    match value {
        0..=1000 => (((usize::from(id) + count - 1) % count) as u8, true),
        3000..=u16::MAX => (((usize::from(id) + 1) % count) as u8, true),
        _ => (id, false),
    }
}

fn snake_direction(sample: InputSample, current: Direction) -> Direction {
    // The original board orientation and Y-over-X diagonal priority are intentional.
    if sample.y > 3000 {
        Direction::Right
    } else if sample.y < 1000 {
        Direction::Left
    } else if sample.x > 3000 {
        Direction::Up
    } else if sample.x < 1000 {
        Direction::Down
    } else {
        current
    }
}
