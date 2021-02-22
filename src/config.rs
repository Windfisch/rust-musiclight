// definitions for the FFT
pub const BLOCK_LEN: usize = 512;
pub const SAMP_RATE: f32   = 48000.0;

// samples read from stdin per update
pub const SAMPLES_PER_UPDATE: usize = BLOCK_LEN/2;

// LED configuration
pub const NUM_STRIPS:         usize =  8;
pub const NUM_LEDS_PER_STRIP: usize = 16;

pub const NUM_LEDS_TOTAL: usize = NUM_STRIPS * NUM_LEDS_PER_STRIP;

// network configuration
pub const UDP_SERVER_ADDR: &str = "192.168.23.118:2703";
