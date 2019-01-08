#[repr(C)]
#[derive(Default, Debug, Copy, Clone)]
pub struct TrapFrame {
    pub elr: u64,      // 0
    pub spsr: u64,     //
    pub sp: u64,       // 16
    pub tpidr: u64,    //
    pub q0: u128,      // 32
    pub q1: u128,      //
    pub q2: u128,      // 64
    pub q3: u128,      //
    pub q4: u128,      // 96
    pub q5: u128,      //
    pub q6: u128,      // 128
    pub q7: u128,      //
    pub q8: u128,      // 160
    pub q9: u128,      //
    pub q10: u128,     // 192
    pub q11: u128,     //
    pub q12: u128,     // 224
    pub q13: u128,     //
    pub q14: u128,     // 256
    pub q15: u128,     //
    pub q16: u128,     // 288
    pub q17: u128,     //
    pub q18: u128,     // 320
    pub q19: u128,     //
    pub q20: u128,     // 352
    pub q21: u128,     //
    pub q22: u128,     // 384
    pub q23: u128,     //
    pub q24: u128,     // 416
    pub q25: u128,     //
    pub q26: u128,     // 448
    pub q27: u128,     //
    pub q28: u128,     // 480
    pub q29: u128,     //
    pub q30: u128,     // 512    0
    pub q31: u128,     //
    pub x1: u64,       // 544    32
    pub x2: u64,       //
    pub x3: u64,       // 560    48
    pub x4: u64,       //
    pub x5: u64,       // 576    64
    pub x6: u64,       //
    pub x7: u64,       // 592    80
    pub x8: u64,       //
    pub x9: u64,       // 608    96
    pub x10: u64,      //
    pub x11: u64,      // 624    112
    pub x12: u64,      //
    pub x13: u64,      // 640    128
    pub x14: u64,      //
    pub x15: u64,      // 656    144
    pub x16: u64,      //
    pub x17: u64,      // 672    160
    pub x18: u64,      //
    pub x19: u64,      // 688    176
    pub x20: u64,      //
    pub x21: u64,      // 704    192
    pub x22: u64,      //
    pub x23: u64,      // 720    208
    pub x24: u64,      //
    pub x25: u64,      // 736    224
    pub x26: u64,      //
    pub x27: u64,      // 752    240
    pub x28: u64,      //
    pub x29: u64,      // 768    256
    pub reserved: u64, //
    pub x30: u64,      // 784    272
    pub x0: u64,       //
}
