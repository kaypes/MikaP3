//! Short, interruptible sounds and a bounded mailbox for collision feedback.

use super::Note;
use crate::breakout::game::Collision;
use heapless::Deque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundEffect {
    Wall,
    Paddle,
    Brick,
}

impl From<Collision> for SoundEffect {
    fn from(collision: Collision) -> Self {
        match collision {
            Collision::Wall => Self::Wall,
            Collision::Paddle => Self::Paddle,
            Collision::Brick => Self::Brick,
        }
    }
}

impl SoundEffect {
    pub fn track(self) -> &'static [Note] {
        const WALL: &[Note] = &[Note {
            freq: 600,
            vol: 90,
            duration_ms: 20,
        }];
        const PADDLE: &[Note] = &[Note {
            freq: 900,
            vol: 90,
            duration_ms: 35,
        }];
        const BRICK: &[Note] = &[
            Note {
                freq: 1300,
                vol: 90,
                duration_ms: 25,
            },
            Note {
                freq: 1700,
                vol: 90,
                duration_ms: 25,
            },
        ];
        match self {
            Self::Wall => WALL,
            Self::Paddle => PADDLE,
            Self::Brick => BRICK,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectRequest {
    pub session: u64,
    pub effect: SoundEffect,
}

pub struct EffectQueue {
    pending: Deque<EffectRequest, 8>,
}

impl Default for EffectQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectQueue {
    pub const fn new() -> Self {
        Self {
            pending: Deque::new(),
        }
    }

    pub fn push(&mut self, effect: EffectRequest) {
        if self.pending.is_full() {
            self.pending.pop_front();
        }
        self.pending.push_back(effect).unwrap();
    }

    pub fn take_latest(&mut self, session: u64) -> Option<EffectRequest> {
        let mut latest = None;
        while let Some(effect) = self.pending.pop_front() {
            if effect.session == session {
                latest = Some(effect);
            }
        }
        latest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_mailbox_stays_bounded_and_retains_the_newest_feedback() {
        let mut queue = EffectQueue::new();
        for session in 0..20 {
            queue.push(EffectRequest {
                session,
                effect: SoundEffect::Wall,
            });
        }
        assert_eq!(queue.pending.len(), 8);
        assert_eq!(queue.take_latest(19).unwrap().session, 19);
        assert!(queue.pending.is_empty());
    }

    #[test]
    fn old_sessions_are_discarded_and_latest_matching_effect_wins() {
        let mut queue = EffectQueue::new();
        queue.push(EffectRequest {
            session: 1,
            effect: SoundEffect::Brick,
        });
        queue.push(EffectRequest {
            session: 2,
            effect: SoundEffect::Wall,
        });
        queue.push(EffectRequest {
            session: 2,
            effect: SoundEffect::Paddle,
        });
        assert_eq!(queue.take_latest(2).unwrap().effect, SoundEffect::Paddle);
        assert_eq!(queue.take_latest(1), None);
    }

    #[test]
    fn effects_are_distinct_and_no_longer_than_sixty_ms() {
        for effect in [SoundEffect::Wall, SoundEffect::Paddle, SoundEffect::Brick] {
            assert!(
                effect
                    .track()
                    .iter()
                    .map(|note| note.duration_ms)
                    .sum::<u64>()
                    <= 60
            );
            assert!(effect.track().iter().all(|note| note.freq > 0));
        }
        assert_ne!(
            SoundEffect::Wall.track()[0].freq,
            SoundEffect::Paddle.track()[0].freq
        );
        assert_ne!(
            SoundEffect::Paddle.track()[0].freq,
            SoundEffect::Brick.track()[0].freq
        );
    }

    #[test]
    fn replacing_an_effect_drops_and_silences_the_old_output_guard() {
        use crate::audio::{Output, Player, Tone};
        use std::{cell::Cell, rc::Rc};
        #[derive(Clone, Default)]
        struct FakeOutput(Rc<Cell<Option<Tone>>>);
        impl Output for FakeOutput {
            fn set_tone(&mut self, tone: Option<Tone>) {
                self.0.set(tone);
            }
        }
        let a = FakeOutput::default();
        let b = FakeOutput::default();
        let mut old = Player::new(
            a.clone(),
            b.clone(),
            [SoundEffect::Paddle.track(), &[]],
            125_000_000,
            Some(0),
            false,
        )
        .unwrap();
        old.update(0);
        let previous_tone = a.0.get().unwrap();
        drop(old);
        assert_eq!(a.0.get(), None);
        assert_eq!(b.0.get(), None);
        let mut new = Player::new(
            a.clone(),
            b.clone(),
            [SoundEffect::Brick.track(), &[]],
            125_000_000,
            Some(10_000),
            false,
        )
        .unwrap();
        new.update(10_000);
        assert_ne!(a.0.get().unwrap(), previous_tone);
        assert_eq!(b.0.get(), None);
        assert!(new.update(60_000).finished);
        assert_eq!(a.0.get(), None);
    }
}
