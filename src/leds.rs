use embassy_futures::select::select;
use embassy_rp::Peri;
use embassy_rp::clocks::clk_sys_freq;
use embassy_rp::pio::{
    Common, Config, Direction, FifoJoin, Pin as PioPin, ShiftConfig, ShiftDirection, StateMachine,
};
use embassy_time::{Duration, Instant, Timer};
use mikap3::visual::MatrixRenderer;

mod encoding;

#[embassy_executor::task]
pub async fn leds_task(
    mut common: Common<'static, embassy_rp::peripherals::PIO0>,
    mut sm: StateMachine<'static, embassy_rp::peripherals::PIO0, 0>,
    mut dma: Peri<'static, embassy_rp::peripherals::DMA_CH0>,
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

    let mut rx = crate::runtime::subscribe_leds();
    let mut renderer = MatrixRenderer::default();
    loop {
        let update = renderer.update(rx.get().await, Instant::now().as_millis());
        if let Some(frame) = update.frame {
            let words = encoding::encode(&frame);
            sm.tx().dma_push(dma.reborrow(), &words, false).await;
            // Allow the joined FIFO to drain and the strip to latch before another frame.
            Timer::after(Duration::from_micros(600)).await;
        }
        if let Some(deadline) = update.next_wake_ms {
            select(rx.changed(), Timer::at(Instant::from_millis(deadline))).await;
        } else {
            rx.changed().await;
        }
    }
}
