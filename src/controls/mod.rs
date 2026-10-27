//! Debounced gestures and joystick navigation, independent of hardware and screens.

#[cfg(test)]
mod tests;

pub const INPUT_INTERVAL_MS: u64 = 10;
pub const DEBOUNCE_MS: u64 = 20;
pub const LONG_PRESS_MS: u64 = 2000;
pub const MULTI_CLICK_MS: u64 = 500;
pub const REPEAT_DELAY_MS: u64 = 350;
pub const REPEAT_INTERVAL_MS: u64 = 150;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AxisDirection {
    Low,
    #[default]
    Neutral,
    High,
}

#[derive(Default, Clone, Copy)]
pub struct ControlContext {
    pub triple_joy: bool,
    pub allow_ab: bool,
    pub suppress_actions: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ControlFrame {
    pub a_pressed: bool,
    pub b_pressed: bool,
    pub joy_clicked: bool,
    pub joy_long: bool,
    pub joy_triple: bool,
    pub reset_usb: bool,
    pub x: AxisDirection,
    pub y: AxisDirection,
    pub x_step: Option<AxisDirection>,
    pub y_step: Option<AxisDirection>,
}

#[derive(Default)]
struct Button {
    candidate: bool,
    since: u64,
    stable: bool,
    blocked: bool,
}

impl Button {
    fn sample(&mut self, raw: bool, now: u64) -> (bool, bool) {
        if raw != self.candidate {
            self.candidate = raw;
            self.since = now;
        }
        let previous = self.stable;
        if now.saturating_sub(self.since) >= DEBOUNCE_MS {
            self.stable = self.candidate;
            if !self.stable {
                self.blocked = false;
            }
        }
        (
            !previous && self.stable && !self.blocked,
            previous && !self.stable && !self.blocked,
        )
    }

    fn block_until_release(&mut self) {
        self.blocked |= self.stable || self.candidate;
    }
}

struct Axis {
    direction: AxisDirection,
    armed: bool,
    repeat_at: Option<u64>,
}

impl Default for Axis {
    fn default() -> Self {
        Self {
            direction: AxisDirection::Neutral,
            armed: true,
            repeat_at: None,
        }
    }
}

impl Axis {
    fn require_neutral(&mut self) {
        self.armed = false;
        self.direction = AxisDirection::Neutral;
        self.repeat_at = None;
    }

    fn sample(&mut self, value: u16, now: u64) -> Option<AxisDirection> {
        if !self.armed {
            self.armed = (1400..=2600).contains(&value);
            return None;
        }
        let next = match self.direction {
            AxisDirection::Low if value < 1400 => AxisDirection::Low,
            AxisDirection::High if value > 2600 => AxisDirection::High,
            _ if value <= 1000 => AxisDirection::Low,
            _ if value >= 3000 => AxisDirection::High,
            _ => AxisDirection::Neutral,
        };
        if next != self.direction {
            self.direction = next;
            self.repeat_at = (next != AxisDirection::Neutral).then_some(now + REPEAT_DELAY_MS);
            return (next != AxisDirection::Neutral).then_some(next);
        }
        if self.repeat_at.is_some_and(|deadline| now >= deadline) {
            // A late sample emits one step, rather than a burst of missed repeats.
            self.repeat_at = Some(now + REPEAT_INTERVAL_MS);
            return Some(next);
        }
        None
    }
}

#[derive(Default)]
pub struct Controls {
    buttons: [Button; 3],
    x: Axis,
    y: Axis,
    joy_started: Option<u64>,
    joy_consumed: bool,
    joy_clicks: u8,
    last_joy_click: Option<u64>,
    reset_started: Option<u64>,
    reset_latched: bool,
}

impl Controls {
    /// A new ball requires a fresh launch press; movement and global escape stay active.
    pub fn require_b_release(&mut self) {
        self.buttons[1].block_until_release();
    }

    pub fn cancel_triple(&mut self) {
        self.joy_clicks = 0;
        self.last_joy_click = None;
    }

    /// Cancel contextual gestures without interrupting a global USB reset chord.
    pub fn rearm(&mut self) {
        for button in &mut self.buttons {
            button.block_until_release();
        }
        self.joy_started = None;
        self.joy_consumed = false;
        self.cancel_triple();
        self.x.require_neutral();
        self.y.require_neutral();
    }

    pub fn sample(&mut self, raw: InputSample, now: u64, context: ControlContext) -> ControlFrame {
        let a = self.buttons[0].sample(raw.a, now);
        let b = self.buttons[1].sample(raw.b, now);
        let joy = self.buttons[2].sample(raw.joy, now);
        let down = self.buttons.each_ref().map(|button| button.stable);
        let count = down.iter().filter(|&&pressed| pressed).count();
        let mut frame = ControlFrame::default();

        if count == 0 {
            self.reset_latched = false;
        }
        if count == 3 {
            let start = *self.reset_started.get_or_insert(now);
            if !self.reset_latched && now.saturating_sub(start) >= LONG_PRESS_MS {
                frame.reset_usb = true;
                self.reset_latched = true;
            }
        } else {
            self.reset_started = None;
        }

        let allowed_pair = context.allow_ab && down == [true, true, false];
        if context.suppress_actions || (count >= 2 && !allowed_pair) {
            self.rearm();
            return frame;
        }

        frame.a_pressed = a.0;
        frame.b_pressed = b.0;
        if !context.triple_joy || a.0 || b.0 {
            self.cancel_triple();
        }
        if joy.0 {
            self.joy_started = Some(now);
            self.joy_consumed = false;
        }
        if let Some(start) = self.joy_started {
            if !self.joy_consumed && now.saturating_sub(start) >= LONG_PRESS_MS {
                frame.joy_long = true;
                self.joy_consumed = true;
                self.cancel_triple();
            }
            if joy.1 {
                if !self.joy_consumed {
                    frame.joy_clicked = true;
                    if context.triple_joy && !a.0 && !b.0 {
                        self.joy_clicks = if self
                            .last_joy_click
                            .is_some_and(|last| now.saturating_sub(last) < MULTI_CLICK_MS)
                        {
                            self.joy_clicks + 1
                        } else {
                            1
                        };
                        self.last_joy_click = Some(now);
                        if self.joy_clicks == 3 {
                            frame.joy_triple = true;
                            self.cancel_triple();
                        }
                    }
                }
                self.joy_started = None;
            }
        }

        frame.x_step = self.x.sample(raw.x, now);
        frame.y_step = self.y.sample(raw.y, now);
        frame.x = self.x.direction;
        frame.y = self.y.direction;
        frame
    }
}
