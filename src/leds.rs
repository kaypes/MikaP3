use embassy_rp::pio::{Common, FifoJoin, ShiftConfig, ShiftDirection, StateMachine, Pin as PioPin, Direction};
use embassy_rp::clocks::clk_sys_freq;
use crate::state::{AppState, STATE};
use crate::arts::{ARTS, LED_MAP};

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

    let mut cfg = embassy_rp::pio::Config::default();
    let loaded = common.load_program(&prg.program);
    cfg.use_program(&loaded, &[&pio_pin]);
    
    let clock_freq = clk_sys_freq();
    let bit_freq = 800_000 * 10;
    let div = (clock_freq / bit_freq) as u8;
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
    let mut last_art_id = 255;

    loop {
        let current_state = rx.get().await;
        
        let art_id = match current_state {
            AppState::Menu { art_id, .. } => art_id,
            AppState::Playing { art_id, .. } => art_id,
        };

        if art_id != last_art_id {
            let pixels = parse_art(art_id);
            
            for p in pixels {
                while sm.tx().full() {
                    core::hint::spin_loop();
                }

                sm.tx().push(p);
            }
            
            last_art_id = art_id;
        }

        rx.changed().await;
    }
}

fn parse_art(id: u8) -> [u32; 25] {
    let mut hardware_data = [0u32; 25];
    let art = ARTS[id as usize];
    
    let b = [10, 5, 3]; 
    
    for y in 0..5 {
        let row = art[y].as_bytes();
        for x in 0..5 {
            let color = match row[x] {
                b'R' => (b[0], 0, 0),
                b'G' => (0, b[1], 0),
                b'B' => (0, 0, b[2]),
                b'Y' => (b[0], b[1], 0),
                b'P' => (b[0], 0, b[2]),
                b'C' => (0, b[1], b[2]),
                b'W' => (b[0], b[1], b[2]),
                b'O' => (b[0], b[1] / 2, 0),
                _    => (0, 0, 0),
            };

            let grb = ((color.1 as u32) << 16) | ((color.0 as u32) << 8) | (color.2 as u32);
            let visual_index = y * 5 + x;
            let physical_led_index = LED_MAP[visual_index];
            
            hardware_data[physical_led_index] = grb << 8;
        }
    }
    hardware_data
}