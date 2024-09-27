pub trait PushConstant {
    fn range(&self) -> (usize, &[u8]);
}

impl PushConstant for () {
    fn range(&self) -> (usize, &[u8]) {
        (0, &[])
    }
}

pub trait BindingSignature {}
