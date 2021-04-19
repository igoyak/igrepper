pub const CASE_INSENSITIVE_PREFIX: &str = "(?i)";

pub static COLOR_PAIR_DEFAULT: i16 = 128;
pub static COLOR_PAIR_INACTIVE_INPUT: i16 = 129;
pub static COLOR_PAIR_ACTIVE_INPUT: i16 = 130;
pub static COLOR_PAIR_BORDER: i16 = 131; // grey
pub static COLOR_PAIR_RED: i16 = 132;

#[allow(dead_code)]
pub static COLOR_PAIR_GREY: i16 = 8;

pub const MAX_MATCH_COLORS: usize = 18;
pub const MATCH_COLORS: [i16; 18] = [
    1,   // red
    190, // green
    27,  // blue
    214, // yellow orange
    206, // pink
    45,  // cyan
    5,   // maroon?
    2,   // green
    99,  // blue
    220, // yellow
    213, // pinkish
    147, // cyanish
    111, 214, 129, 226, 215, 70,
];

pub const CTRL_D: i32 = 'd' as i32 - 0x60;
pub const CTRL_E: i32 = 'e' as i32 - 0x60;
pub const CTRL_G: i32 = 'g' as i32 - 0x60;
pub const CTRL_H: i32 = 'h' as i32 - 0x60;
pub const CTRL_I: i32 = 'i' as i32 - 0x60;
pub const CTRL_L: i32 = 'l' as i32 - 0x60;
pub const CTRL_N: i32 = 'n' as i32 - 0x60;
pub const CTRL_P: i32 = 'p' as i32 - 0x60;
pub const CTRL_R: i32 = 'r' as i32 - 0x60;
pub const CTRL_T: i32 = 't' as i32 - 0x60;
pub const CTRL_U: i32 = 'u' as i32 - 0x60;
pub const CTRL_V: i32 = 'v' as i32 - 0x60;
pub const F1: i32 = 27;
pub const F1_2: i32 = 265;
