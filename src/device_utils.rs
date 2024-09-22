use pollster::FutureExt as _;

pub trait DeviceUtils {
    fn wgpu_try<T, F>(&self, filter: wgpu::ErrorFilter, func: F) -> Result<T, wgpu::Error>
    where
        F: FnOnce(&wgpu::Device) -> T;
}

impl DeviceUtils for wgpu::Device {
    fn wgpu_try<T, F>(&self, filter: wgpu::ErrorFilter, func: F) -> Result<T, wgpu::Error>
    where
        F: FnOnce(&wgpu::Device) -> T,
    {
        self.push_error_scope(filter);
        let result = func(self);
        match self.pop_error_scope().block_on() {
            Some(error) => Err(error),
            None => Ok(result),
        }
    }
}
