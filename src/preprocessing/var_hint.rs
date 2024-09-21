use std::ops::Range;

#[derive(Debug)]
pub enum VarType {
    Float {
        range: Option<Range<f32>>,
        default: Option<f32>,
    },
    Uint {
        range: Option<Range<u32>>,
        default: Option<u32>,
    },
    Sint {
        range: Option<Range<i32>>,
        default: Option<i32>,
    },
    Bool {
        default: Option<bool>,
    },
    Color {
        default: Option<[f32; 4]>,
    },
}

#[derive(Debug)]
pub struct GlobalVarHint {
    pub ty: VarType,
}
