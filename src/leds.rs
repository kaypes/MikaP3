use crate::arts::{ARTS, LED_MAP};
use crate::state::{AppState, STATE};
use embassy_rp::clocks::clk_sys_freq;
use embassy_rp::pio::{
    Common, Config, Direction, FifoJoin, Pin as PioPin, ShiftConfig, ShiftDirection, StateMachine,
};

#[embassy_executor::task]
pub async fn leds_task(
    mut common: Common<'static, embassy_rp::peripherals::PIO0>,
    mut sm: StateMachine<'static, embassy_rp::peripherals::PIO0, 0>,
    pio_pin: PioPin<'static, embassy_rp::peripherals::PIO0>,
) {
    let prg = pio::pio_asm!(
        ".side_set 1",
        ".wrap_target",
        "bitloop:",
        "out x, 1       side 0 [2]",
        "jmp !x do_zero side 1 [1]",
        "do_one:",
        "jmp  bitloop   side 1 [4]",
        "do_zero:",
        "nop            side 0 [4]",
        ".wrap",
    );

    let mut cfg = Config::default();
    let loaded = common.load_program(&prg.program);
    cfg.use_program(&loaded, &[&pio_pin]);

    let clock_freq: u32 = clk_sys_freq();
    let bit_freq: u32 = 800_000 * 10;
    let div: u8 = (clock_freq / bit_freq) as u8;
    cfg.clock_divider = div.into();

    cfg.fifo_join = FifoJoin::TxOnly;
    cfg.shift_out = ShiftConfig {
        auto_fill: true,
        threshold: 24,
        direction: ShiftDirection::Left,
    };

    sm.set_config(&cfg);
    sm.set_pin_dirs(Direction::Out, &[&pio_pin]);
    sm.set_enable(true);

    let mut rx = STATE.receiver().unwrap();
    let mut last_art_id: u8 = 255;

    loop {
        let current_state: AppState = rx.get().await;

        let pixels: Option<[u32; 25]> = match current_state {
            AppState::Menu { art_id, .. } | AppState::Playing { art_id, .. } => {
                if art_id != last_art_id {
                    last_art_id = art_id;
                    Some(parse_art(art_id))
                } else {
                    None
                }
            }

            AppState::Snake(game) => {
                last_art_id = 255;
                let rgb_pixels: [(u8, u8, u8); 25] = crate::snake::render::draw_frame(&game);
                let mut hardware_data: [u32; 25] = [0u32; 25];

                for visual_index in 0..25 {
                    let (r, g, b) = rgb_pixels[visual_index];
                    let grb: u32 = ((g as u32) << 16) | ((r as u32) << 8) | (b as u32);
                    let physical_led_index: usize = LED_MAP[visual_index];
                    hardware_data[physical_led_index] = grb << 8;
                }
                Some(hardware_data)
            }
        };

        if let Some(data) = pixels {
            for p in data {
                while sm.tx().full() {
                    core::hint::spin_loop();
                }

                sm.tx().push(p);
            }
        }

        rx.changed().await;
    }
}

fn parse_art(id: u8) -> [u32; 25] {
    let mut hardware_data: [u32; 25] = [0u32; 25];
    let art: [&str; 5] = ARTS[id as usize];

    let b: [i32; 3] = [10, 5, 3];

    for y in 0..5 {
        let row: &[u8] = art[y].as_bytes();
        for x in 0..5 {
            let color: (i32, i32, i32) = match row[x] {
                b'R' => (b[0], 0, 0),
                b'G' => (0, b[1], 0),
                b'B' => (0, 0, b[2]),
                b'Y' => (b[0], b[1], 0),
                b'P' => (b[0], 0, b[2]),
                b'C' => (0, b[1], b[2]),
                b'W' => (b[0], b[1], b[2]),
                b'O' => (b[0], b[1] / 2, 0),
                b'L' => (b[1], b[2], 0),
                _ => (0, 0, 0),
            };

            let grb: u32 = ((color.1 as u32) << 16) | ((color.0 as u32) << 8) | (color.2 as u32);
            let visual_index: usize = y * 5 + x;
            let physical_led_index: usize = LED_MAP[visual_index];

            hardware_data[physical_led_index] = grb << 8;
        }
    }
    hardware_data
}
