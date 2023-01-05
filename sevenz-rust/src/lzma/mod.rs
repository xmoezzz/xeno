mod decoder;
mod lz;
mod lzma2_reader;
mod range_codec;
mod state;
mod lzma_reader;
pub use lzma2_reader::LZMA2Reader;
pub use lzma_reader::LZMAReader;
pub use lzma2_reader::get_memery_usage as lzma2_get_memery_usage;
pub use lzma_reader::get_memery_usage as lzma_get_memery_usage;
pub use lzma_reader::get_memery_usage_by_props as lzma_get_memery_usage_by_props;

use state::*;

pub const DICT_SIZE_MIN: u32 = 4096;
pub const DICT_SIZE_MAX: u32 = u32::MAX & !(15 as u32);

const LOW_SYMBOLS: usize = 1 << 3;
const MID_SYMBOLS: usize = 1 << 3;
const HIGH_SYMBOLS: usize = 1 << 8;

const POS_STATES_MAX: usize = 1 << 4;
const MATCH_LEN_MIN: usize = 2;
const MATCH_LEN_MAX: usize = MATCH_LEN_MIN + LOW_SYMBOLS + MID_SYMBOLS + HIGH_SYMBOLS - 1;

const DIST_STATES: usize = 4;
const DIST_SLOTS: usize = 1 << 6;
const DIST_MODEL_START: usize = 4;
const DIST_MODEL_END: usize = 14;
const FULL_DISTANCES: usize = 1 << (DIST_MODEL_END / 2);

const ALIGN_BITS: usize = 4;
const ALIGN_SIZE: usize = 1 << ALIGN_BITS;
const ALIGN_MASK: usize = ALIGN_SIZE - 1;

const REPS: usize = 4;

const SHIFT_BITS: u32 = 8;
const TOP_MASK: u32 = 0xFF000000;
const BIT_MODEL_TOTAL_BITS: u32 = 11;
const BIT_MODEL_TOTAL: u32 = 1 << BIT_MODEL_TOTAL_BITS;
const PROB_INIT: u16 = (BIT_MODEL_TOTAL / 2) as u16;
const MOVE_BITS: u32 = 5;
pub struct LZMACoder {
    pub(crate) pos_mask: u32,
    pub(crate) reps: [i32; REPS],
    pub(crate) state: State,
    pub(crate) is_match: [[u16; POS_STATES_MAX]; state::STATES],
    pub(crate) is_rep: [u16; state::STATES],
    pub(crate) is_rep0: [u16; state::STATES],
    pub(crate) is_rep1: [u16; state::STATES],
    pub(crate) is_rep2: [u16; state::STATES],
    pub(crate) is_rep0_long: [[u16; POS_STATES_MAX]; state::STATES],
    pub(crate) dist_slots: [[u16; DIST_SLOTS]; DIST_STATES],
    pub(crate) dist_special: (
        [u16; 2],
        [u16; 2],
        [u16; 4],
        [u16; 4],
        [u16; 8],
        [u16; 8],
        [u16; 16],
        [u16; 16],
        [u16; 32],
        [u16; 32],
    ),
    dist_align: [u16; ALIGN_SIZE],
}

pub(crate) fn coder_get_dict_size(len: usize) -> usize {
    if len < DIST_STATES + MATCH_LEN_MIN {
        len - MATCH_LEN_MIN
    } else {
        DIST_STATES - 1
    }
}

impl LZMACoder {
    pub fn new(pb: usize) -> Self {
        let mut c = Self {
            pos_mask: (1 << pb) - 1,
            reps: Default::default(),
            state: Default::default(),
            is_match: Default::default(),
            is_rep: Default::default(),
            is_rep0: Default::default(),
            is_rep1: Default::default(),
            is_rep2: Default::default(),
            is_rep0_long: Default::default(),
            dist_slots: [[Default::default(); DIST_SLOTS]; DIST_STATES],
            dist_special: Default::default(),
            dist_align: Default::default(),
        };
        c.reset();
        c
    }

    pub fn reset(&mut self) {
        self.reps = [0; REPS];
        self.state.reset();
        for ele in self.is_match.iter_mut() {
            init_probs(ele);
        }
        init_probs(&mut self.is_rep);
        init_probs(&mut self.is_rep0);
        init_probs(&mut self.is_rep1);
        init_probs(&mut self.is_rep2);

        for ele in self.is_rep0_long.iter_mut() {
            init_probs(ele);
        }
        for ele in self.dist_slots.iter_mut() {
            init_probs(ele);
        }
        init_probs(&mut self.dist_special.0);
        init_probs(&mut self.dist_special.1);
        init_probs(&mut self.dist_special.2);
        init_probs(&mut self.dist_special.3);
        init_probs(&mut self.dist_special.4);
        init_probs(&mut self.dist_special.5);
        init_probs(&mut self.dist_special.6);
        init_probs(&mut self.dist_special.7);
        init_probs(&mut self.dist_special.8);
        init_probs(&mut self.dist_special.9);
        init_probs(&mut self.dist_align);
    }

    pub fn get_dist_special(&mut self, i: usize) -> &mut [u16] {
        let ds: &mut [&mut [u16]] = &mut [
            &mut self.dist_special.0,
            &mut self.dist_special.1,
            &mut self.dist_special.2,
            &mut self.dist_special.3,
            &mut self.dist_special.4,
            &mut self.dist_special.5,
            &mut self.dist_special.6,
            &mut self.dist_special.7,
            &mut self.dist_special.8,
            &mut self.dist_special.9,
        ];
        let len = ds[i].len();
        let d = ds[i].as_mut_ptr();
        unsafe { std::slice::from_raw_parts_mut(d, len) }
    }
}

#[inline(always)]
pub(crate) fn init_probs(probs: &mut [u16]) {
    probs.fill(PROB_INIT);
}

pub(crate) struct LiteralCoder {
    lc: u32,
    literal_pos_mask: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct LiteralSubcoder {
    probs: [u16; 0x300],
}

impl LiteralSubcoder {
    pub fn new() -> Self {
        let mut probs = [0u16; 0x300];
        // init_probs(&mut probs);
        Self { probs }
    }
    pub fn reset(&mut self) {
        init_probs(&mut self.probs);
    }
}

impl LiteralCoder {
    pub fn new(lc: u32, lp: u32) -> Self {
        Self {
            lc,
            literal_pos_mask: (1 << lp) - 1,
        }
    }
    pub(crate) fn get_sub_coder_index(&self, prev_byte: u32, pos: u32) -> u32 {
        let low = prev_byte >> (8 - self.lc);
        let high = (pos & self.literal_pos_mask) << self.lc;
        low + high
    }
}

pub(crate) struct LengthCoder {
    choice: [u16; 2],
    low: [[u16; LOW_SYMBOLS]; POS_STATES_MAX],
    mid: [[u16; MID_SYMBOLS]; POS_STATES_MAX],
    high: [u16; HIGH_SYMBOLS],
}

impl LengthCoder {
    pub fn new() -> Self {
        Self {
            choice: Default::default(),
            low: Default::default(),
            mid: Default::default(),
            high: [0; HIGH_SYMBOLS],
        }
    }

    pub fn reset(&mut self) {
        init_probs(&mut self.choice);
        for ele in self.low.iter_mut() {
            init_probs(ele);
        }
        for ele in self.mid.iter_mut() {
            init_probs(ele);
        }
        init_probs(&mut self.high);
    }
}