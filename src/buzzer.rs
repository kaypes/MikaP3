#![no_std]

use embassy_rp::pwm::{Config, Pwm};
use embassy_time::{Duration, Timer, Instant};

#[derive(Copy, Clone)]
pub struct Note {
    pub freq_a: u32,
    pub vol_a: u8,
    pub freq_b: u32,
    pub vol_b: u8,
    pub duration_ms: u64,
}

pub async fn play_melody(pwm_a: &mut Pwm<'_>, pwm_b: &mut Pwm<'_>, melody: &[Note]) {
    let mut active_freq_a = 0;
    let mut active_initial_vol_a = 0;
    let mut current_vol_a: u8 = 0;

    let mut active_freq_b = 0;
    let mut active_initial_vol_b = 0;
    let mut current_vol_b: u8 = 0;

    // CONFIGURAÇÕES DO EFEITO SONORO
    // Taxa de atualização (10ms é suave e não sobrecarrega o microcontrolador)
    let tick_ms = 10;

    // Fator de decaimento (Decay).
    // - 16 a 32 geram um efeito legal parecido com violão/piano.
    // - Valores maiores (ex: 64) deixam o som sustentado por mais tempo (como um órgão).
    let decay_divisor = 32;

    for i in 0..melody.len() {
        let note = &melody[i];
        let next_note = melody.get(i + 1);

        // Verifica pausas artificiais do script Python para fazer o Legato
        let is_artificial_gap_a = note.freq_a == 0
            && note.duration_ms <= 5
            && next_note.map_or(false, |n| n.freq_a == active_freq_a);

        let is_artificial_gap_b = note.freq_b == 0
            && note.duration_ms <= 5
            && next_note.map_or(false, |n| n.freq_b == active_freq_b);

        // BUZZER A: Inicia uma nova nota e reseta o volume se a nota mudou (Ignorando gaps)
        if !is_artificial_gap_a && (note.freq_a != active_freq_a || note.vol_a != active_initial_vol_a) {
            active_freq_a = note.freq_a;
            active_initial_vol_a = note.vol_a;
            current_vol_a = note.vol_a; // Reseta o volume para o ataque inicial
        }

        // BUZZER B: Mesma lógica
        if !is_artificial_gap_b && (note.freq_b != active_freq_b || note.vol_b != active_initial_vol_b) {
            active_freq_b = note.freq_b;
            active_initial_vol_b = note.vol_b;
            current_vol_b = note.vol_b;
        }

        // ENVELOPE LOOP: Toca a nota atualizando o decaimento em "ticks"
        let mut time_left = note.duration_ms;

        while time_left > 0 {
            // Dorme 10ms (ou o tempo que sobrar, se for menor)
            let sleep_time = if time_left > tick_ms { tick_ms } else { time_left };

            // --- Aplica o volume atual no Buzzer A ---
            let mut config_a = Config::default();
            config_a.divider = 125.into();
            if active_freq_a > 0 && current_vol_a > 0 {
                let top_a = (1_000_000 / active_freq_a) as u16;
                config_a.top = top_a;
                // Duty cycle diminui conforme current_vol_a decai
                let duty_cycle = (top_a as u32 * current_vol_a as u32) / (2 * 127);
                config_a.compare_b = duty_cycle as u16;
            } else {
                config_a.top = 1000;
                config_a.compare_b = 0; // Mudo
            }
            pwm_a.set_config(&config_a);

            // --- Aplica o volume atual no Buzzer B ---
            let mut config_b = Config::default();
            config_b.divider = 125.into();
            if active_freq_b > 0 && current_vol_b > 0 {
                let top_b = (1_000_000 / active_freq_b) as u16;
                config_b.top = top_b;
                let duty_cycle = (top_b as u32 * current_vol_b as u32) / (2 * 127);
                config_b.compare_a = duty_cycle as u16;
            } else {
                config_b.top = 1000;
                config_b.compare_a = 0; // Mudo
            }
            pwm_b.set_config(&config_b);

            // Espera o tick passar
            Timer::after(Duration::from_millis(sleep_time)).await;
            time_left -= sleep_time;

            // --- Calcula a perda de volume para o próximo tick (Decaimento Exponencial) ---
            if active_freq_a > 0 {
                // Diminui uma fração do volume. ".max(1)" garante que não fique preso por divisão de inteiros
                let drop_a = (current_vol_a / decay_divisor).max(1);
                current_vol_a = current_vol_a.saturating_sub(drop_a);
            }
            if active_freq_b > 0 {
                let drop_b = (current_vol_b / decay_divisor).max(1);
                current_vol_b = current_vol_b.saturating_sub(drop_b);
            }
        }
    }

    // Silencia tudo no final da música
    let mut mute = Config::default();
    mute.compare_a = 0;
    mute.compare_b = 0;
    pwm_a.set_config(&mute);
    pwm_b.set_config(&mute);
}