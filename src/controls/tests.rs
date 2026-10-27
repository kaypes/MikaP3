use super::*;

fn neutral() -> InputSample {
    InputSample::default()
}
fn joy() -> InputSample {
    InputSample {
        joy: true,
        ..neutral()
    }
}
fn context() -> ControlContext {
    ControlContext {
        triple_joy: true,
        ..Default::default()
    }
}
fn confirm(
    controls: &mut Controls,
    at: u64,
    raw: InputSample,
    ctx: ControlContext,
) -> ControlFrame {
    controls.sample(raw, at, ctx);
    controls.sample(raw, at + DEBOUNCE_MS, ctx)
}
fn click(controls: &mut Controls, at: u64, ctx: ControlContext) -> ControlFrame {
    confirm(controls, at, joy(), ctx);
    confirm(controls, at + 40, neutral(), ctx)
}

#[test]
fn button_requires_twenty_ms_of_continuous_stability_and_does_not_repeat() {
    let mut controls = Controls::default();
    let a = InputSample {
        a: true,
        ..neutral()
    };
    assert!(!controls.sample(a, 0, context()).a_pressed);
    assert!(!controls.sample(neutral(), 10, context()).a_pressed);
    assert!(!controls.sample(a, 15, context()).a_pressed);
    assert!(!controls.sample(a, 34, context()).a_pressed);
    assert!(controls.sample(a, 35, context()).a_pressed);
    assert!(!controls.sample(a, 5000, context()).a_pressed);
    confirm(&mut controls, 5010, neutral(), context());
    assert!(confirm(&mut controls, 5050, a, context()).a_pressed);
}

#[test]
fn a_shorter_than_debounce_tap_is_rejected() {
    let mut controls = Controls::default();
    controls.sample(joy(), 0, context());
    controls.sample(neutral(), 10, context());
    assert_eq!(
        controls.sample(neutral(), 30, context()),
        ControlFrame::default()
    );
}

#[test]
fn joystick_click_is_emitted_once_on_confirmed_release() {
    let mut controls = Controls::default();
    let pressed = confirm(&mut controls, 0, joy(), context());
    assert!(!pressed.joy_clicked && !pressed.joy_long);
    assert!(!controls.sample(neutral(), 40, context()).joy_clicked);
    assert!(controls.sample(neutral(), 60, context()).joy_clicked);
    assert!(!controls.sample(neutral(), 100, context()).joy_clicked);
}

#[test]
fn long_press_fires_once_consumes_click_and_rearms_after_release() {
    let mut controls = Controls::default();
    confirm(&mut controls, 0, joy(), context());
    assert!(!controls.sample(joy(), 2019, context()).joy_long);
    assert!(controls.sample(joy(), 2020, context()).joy_long);
    assert!(!controls.sample(joy(), 5000, context()).joy_long);
    let released = confirm(&mut controls, 5010, neutral(), context());
    assert!(!released.joy_clicked && !released.joy_triple);
    confirm(&mut controls, 6000, joy(), context());
    assert!(controls.sample(joy(), 8020, context()).joy_long);
}

#[test]
fn releasing_exactly_at_long_press_boundary_is_not_a_click() {
    let mut controls = Controls::default();
    confirm(&mut controls, 0, joy(), context());
    let released = confirm(&mut controls, 2000, neutral(), context());
    assert!(released.joy_long);
    assert!(!released.joy_clicked);
}

#[test]
fn triple_click_requires_three_complete_clicks_and_resets_after_firing() {
    let mut controls = Controls::default();
    assert!(!click(&mut controls, 0, context()).joy_triple);
    assert!(!click(&mut controls, 100, context()).joy_triple);
    assert!(click(&mut controls, 200, context()).joy_triple);
    assert!(!click(&mut controls, 300, context()).joy_triple);
}

#[test]
fn five_hundred_ms_between_releases_starts_a_new_sequence() {
    let mut controls = Controls::default();
    click(&mut controls, 0, context()); // release at 60
    assert!(!click(&mut controls, 500, context()).joy_triple); // release at 560
    assert!(!click(&mut controls, 600, context()).joy_triple);
    assert!(click(&mut controls, 700, context()).joy_triple);
}

#[test]
fn long_press_and_another_button_interrupt_multi_click() {
    let mut controls = Controls::default();
    click(&mut controls, 0, context());
    click(&mut controls, 100, context());
    confirm(&mut controls, 200, joy(), context());
    assert!(controls.sample(joy(), 2220, context()).joy_long);
    confirm(&mut controls, 2300, neutral(), context());
    assert!(!click(&mut controls, 2400, context()).joy_triple);
    assert!(!click(&mut controls, 2500, context()).joy_triple);
    confirm(
        &mut controls,
        2600,
        InputSample {
            a: true,
            ..neutral()
        },
        context(),
    );
    confirm(&mut controls, 2640, neutral(), context());
    assert!(!click(&mut controls, 2700, context()).joy_triple);
}

#[test]
fn disabled_triple_context_and_explicit_cancellation_discard_partial_sequences() {
    let mut controls = Controls::default();
    click(&mut controls, 0, context());
    click(&mut controls, 100, context());
    controls.sample(neutral(), 170, ControlContext::default());
    assert!(!click(&mut controls, 200, context()).joy_triple);
    click(&mut controls, 300, context());
    controls.cancel_triple();
    assert!(!click(&mut controls, 400, context()).joy_triple);
}

#[test]
fn illegal_chord_blocks_actions_until_each_held_button_is_released() {
    let mut controls = Controls::default();
    let chord = InputSample {
        a: true,
        joy: true,
        ..neutral()
    };
    assert_eq!(
        confirm(&mut controls, 0, chord, context()),
        ControlFrame::default()
    );
    confirm(&mut controls, 100, joy(), context());
    assert!(!controls.sample(joy(), 3000, context()).joy_long);
    assert!(!confirm(&mut controls, 3100, neutral(), context()).joy_clicked);
    confirm(&mut controls, 3200, joy(), context());
    assert!(controls.sample(joy(), 5220, context()).joy_long);
}

#[test]
fn simultaneous_ab_is_available_only_in_the_flappy_context() {
    let pair = InputSample {
        a: true,
        b: true,
        ..neutral()
    };
    let mut controls = Controls::default();
    assert_eq!(
        confirm(&mut controls, 0, pair, context()),
        ControlFrame::default()
    );
    let mut controls = Controls::default();
    let ctx = ControlContext {
        allow_ab: true,
        ..Default::default()
    };
    let frame = confirm(&mut controls, 0, pair, ctx);
    assert!(frame.a_pressed && frame.b_pressed);
    let held = controls.sample(pair, 100, ctx);
    assert!(!held.a_pressed && !held.b_pressed);
}

#[test]
fn usb_reset_has_priority_and_latches_until_all_buttons_are_up() {
    let mut controls = Controls::default();
    let all = InputSample {
        a: true,
        b: true,
        joy: true,
        ..neutral()
    };
    confirm(&mut controls, 0, all, context());
    assert!(!controls.sample(all, 2019, context()).reset_usb);
    let reset = controls.sample(all, 2020, context());
    assert!(reset.reset_usb);
    assert!(!reset.a_pressed && !reset.b_pressed && !reset.joy_long);
    assert!(!controls.sample(all, 5000, context()).reset_usb);
    confirm(&mut controls, 5010, joy(), context());
    confirm(&mut controls, 5100, all, context());
    assert!(!controls.sample(all, 7120, context()).reset_usb);
    confirm(&mut controls, 7200, neutral(), context());
    confirm(&mut controls, 7300, all, context());
    assert!(controls.sample(all, 9320, context()).reset_usb);
}

#[test]
fn interrupted_reset_does_not_turn_into_a_joystick_long_press_or_click() {
    let mut controls = Controls::default();
    let all = InputSample {
        a: true,
        b: true,
        joy: true,
        ..neutral()
    };
    confirm(&mut controls, 0, all, context());
    confirm(&mut controls, 1000, joy(), context());
    let held = controls.sample(joy(), 4000, context());
    assert!(!held.reset_usb && !held.joy_long);
    assert!(!confirm(&mut controls, 4100, neutral(), context()).joy_clicked);
}

#[test]
fn reset_remains_available_when_normal_actions_are_suppressed() {
    let mut controls = Controls::default();
    let ctx = ControlContext {
        suppress_actions: true,
        ..context()
    };
    let all = InputSample {
        a: true,
        b: true,
        joy: true,
        ..neutral()
    };
    confirm(&mut controls, 0, all, ctx);
    assert!(controls.sample(all, 2020, ctx).reset_usb);
}

#[test]
fn screen_rearm_also_blocks_a_press_still_waiting_for_debounce() {
    let mut controls = Controls::default();
    let b = InputSample {
        b: true,
        ..neutral()
    };
    controls.sample(b, 0, context());
    controls.rearm();
    assert!(!controls.sample(b, 20, context()).b_pressed);
    assert!(!controls.sample(b, 2000, context()).b_pressed);
    confirm(&mut controls, 2010, neutral(), context());
    assert!(confirm(&mut controls, 2050, b, context()).b_pressed);
}

#[test]
fn axes_step_immediately_and_repeat_independently_without_catchup_bursts() {
    let mut controls = Controls::default();
    let x = InputSample {
        x: 4095,
        ..neutral()
    };
    assert_eq!(
        controls.sample(x, 0, context()).x_step,
        Some(AxisDirection::High)
    );
    let both = InputSample { y: 0, ..x };
    let frame = controls.sample(both, 150, context());
    assert_eq!(
        (frame.x_step, frame.y_step),
        (None, Some(AxisDirection::Low))
    );
    let frame = controls.sample(both, 350, context());
    assert_eq!(
        (frame.x_step, frame.y_step),
        (Some(AxisDirection::High), None)
    );
    let frame = controls.sample(both, 500, context());
    assert_eq!(
        (frame.x_step, frame.y_step),
        (Some(AxisDirection::High), Some(AxisDirection::Low))
    );
    assert!(controls.sample(both, 649, context()).x_step.is_none());
    assert!(controls.sample(both, 650, context()).x_step.is_some());
    assert!(controls.sample(both, 2000, context()).x_step.is_some());
    assert!(controls.sample(both, 2001, context()).x_step.is_none());
    assert!(controls.sample(both, 2150, context()).x_step.is_some());
}

#[test]
fn hysteresis_prevents_threshold_jitter_and_direction_reversal_is_immediate() {
    let mut controls = Controls::default();
    let mut raw = neutral();
    for (now, value, direction, step) in [
        (0, 1000, AxisDirection::Low, true),
        (10, 1399, AxisDirection::Low, false),
        (20, 1400, AxisDirection::Neutral, false),
        (30, 2999, AxisDirection::Neutral, false),
        (40, 3000, AxisDirection::High, true),
        (50, 2601, AxisDirection::High, false),
        (60, 2600, AxisDirection::Neutral, false),
        (70, 0, AxisDirection::Low, true),
        (80, 4095, AxisDirection::High, true),
    ] {
        raw.x = value;
        let frame = controls.sample(raw, now, context());
        assert_eq!(frame.x, direction);
        assert_eq!(frame.x_step.is_some(), step);
    }
}

#[test]
fn screen_rearm_requires_axes_to_return_to_center_before_navigation() {
    let mut controls = Controls::default();
    let x = InputSample { x: 0, ..neutral() };
    controls.sample(x, 0, context());
    controls.rearm();
    assert_eq!(
        controls.sample(x, 5000, context()).x,
        AxisDirection::Neutral
    );
    assert!(
        controls
            .sample(
                InputSample {
                    x: 1200,
                    ..neutral()
                },
                5010,
                context()
            )
            .x_step
            .is_none()
    );
    controls.sample(neutral(), 5020, context());
    assert_eq!(
        controls.sample(x, 5030, context()).x_step,
        Some(AxisDirection::Low)
    );
}
