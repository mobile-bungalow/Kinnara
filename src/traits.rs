pub trait PushConstant {
    fn range<'a>(&'a self) -> (usize, &'a [u8]);
}

impl PushConstant for () {
    fn range<'a>(&'a self) -> (usize, &'a [u8]) {
        (0, &[])
    }
}

pub trait BindingSignature {}
