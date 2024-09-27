
pub trait DeviceUtils {
    fn wgpu_try<T, F>(&self, filter: wgpu::ErrorFilter, func: F) -> Result<T, wgpu::Error>
    where
        F: FnOnce(&wgpu::Device) -> T;

    fn buffer_view<T, F>(&self, buffer: &wgpu::Buffer, func: F) -> T
    where
        F: FnOnce(Option<&[u8]>) -> T;
}

impl DeviceUtils for wgpu::Device {
    fn wgpu_try<T, F>(&self, filter: wgpu::ErrorFilter, func: F) -> Result<T, wgpu::Error>
    where
        F: FnOnce(&wgpu::Device) -> T,
    {
        //TODO: Naga PANICS! when the validation error is non-global.
        // this makes this utility aggravating.

        //self.push_error_scope(filter);
        let result = func(self);
        //match self.pop_error_scope().block_on() {
        //    Some(error) => Err(dbg!(error)),
        //    None => Ok(result),
        //}
        Ok(result)
    }

    /// funs the function on Some(&[u8]) if it the buffer can be mapped,
    /// and none otherwise
    fn buffer_view<T, F>(&self, buffer: &wgpu::Buffer, func: F) -> T
    where
        F: FnOnce(Option<&[u8]>) -> T,
    {
        if buffer.usage().contains(wgpu::BufferUsages::MAP_READ) {
            let buffer_slice = buffer.slice(..);
            buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
            self.poll(wgpu::Maintain::Wait);
            let results = buffer_slice.get_mapped_range();

            let res = func(Some(results.as_ref()));

            std::mem::drop(results);
            buffer.unmap();

            res
        } else {
            func(None)
        }
    }
}
