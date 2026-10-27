use crate::breakout::game::BreakoutGame;
use crate::flappy::game::FlappyGame;
use crate::snake::game::SnakeGame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameId {
    Snake,
    FlappyBird,
    Breakout,
}

impl GameId {
    pub const ALL: [Self; 3] = [Self::Snake, Self::FlappyBird, Self::Breakout];

    pub fn name(self) -> &'static str {
        match self {
            Self::Snake => "> COBRINHA",
            Self::FlappyBird => "> FLAPPY BIRD",
            Self::Breakout => "> BREAKOUT",
        }
    }

    pub fn cycle(self, forward: bool) -> Self {
        let index = Self::ALL.iter().position(|&game| game == self).unwrap();
        let next = if forward {
            index + 1
        } else {
            index + Self::ALL.len() - 1
        };
        Self::ALL[next % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Playing,
    GamesMenu,
    Snake,
    FlappyBird,
    Breakout,
    EasterEgg,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    GamesMenu {
        game_id: GameId,
        with_music: bool,
    },
    Snake(SnakeGame, bool),
    FlappyBird(FlappyGame, bool),
    Breakout(BreakoutGame, bool),
    EasterEgg,
}

impl AppState {
    pub const INITIAL: Self = Self::Menu {
        song_id: 0,
        art_id: 0,
    };

    pub fn screen(self) -> Screen {
        match self {
            Self::Menu { .. } => Screen::Menu,
            Self::Playing { .. } => Screen::Playing,
            Self::GamesMenu { .. } => Screen::GamesMenu,
            Self::Snake(..) => Screen::Snake,
            Self::FlappyBird(..) => Screen::FlappyBird,
            Self::Breakout(..) => Screen::Breakout,
            Self::EasterEgg => Screen::EasterEgg,
        }
    }
}
