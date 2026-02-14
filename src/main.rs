#![no_std]
#![no_main]

use panic_halt as _;
use cortex_m_rt as _;
use embassy_rp as _;

use embassy_executor::Spawner;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
}
