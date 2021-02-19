// definitions for the FFT
pub const BLOCK_LEN: usize = 512;
pub const SAMP_RATE: f32   = 48000.0;

// samples read from stdin per update
pub const SAMPLES_PER_UPDATE: usize = BLOCK_LEN/2;
