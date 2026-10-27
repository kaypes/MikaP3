//! RP2040 board wiring and WS2812 GRB words. No rendering decisions live here.

const LED_MAP: [usize; 25] = [
    24, 23, 22, 21, 20, 15, 16, 17, 18, 19, 14, 13, 12, 11, 10, 5, 6, 7, 8, 9, 4, 3, 2, 1, 0,
];

pub fn encode(frame: &[(u8, u8, u8); 25]) -> [u32; 25] {
    let mut words = [0; 25];
    for (visual_index, &(r, g, b)) in frame.iter().enumerate() {
        let grb = (u32::from(g) << 16) | (u32::from(r) << 8) | u32::from(b);
        words[LED_MAP[visual_index]] = grb << 8;
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wiring_is_a_permutation_of_every_led() {
        let mut seen = [false; 25];
        for physical in LED_MAP {
            assert!(physical < 25 && !seen[physical]);
            seen[physical] = true;
        }
        assert!(seen.iter().all(|&value| value));
    }

    #[test]
    fn visual_rows_follow_board_orientation_and_grb_bit_order() {
        let mut frame = [(0, 0, 0); 25];
        frame[0] = (0x12, 0x34, 0x56);
        frame[4] = (1, 2, 3);
        frame[5] = (4, 5, 6);
        frame[9] = (7, 8, 9);
        frame[20] = (10, 11, 12);
        frame[24] = (13, 14, 15);
        let words = encode(&frame);
        assert_eq!(words[24], 0x34125600);
        assert_eq!(words[20], 0x02010300);
        assert_eq!(words[15], 0x05040600);
        assert_eq!(words[19], 0x08070900);
        assert_eq!(words[4], 0x0b0a0c00);
        assert_eq!(words[0], 0x0e0d0f00);
        assert_eq!(words.iter().filter(|&&word| word != 0).count(), 6);
    }
}
