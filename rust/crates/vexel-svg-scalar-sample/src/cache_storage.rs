//! Private allocation capability; production uses only CacheTransaction.
use vexel_scene::*;
pub(crate) trait Storage {
    fn buffer<T: JobElement>(&mut self, pool: RasterPool, cap: usize, c: &mut dyn WorkControl) -> Result<JobBuffer<T>>;
    fn reserve<T: JobElement>(&mut self, h: JobBuffer<T>, n: usize, c: &mut dyn WorkControl) -> Result<()>;
    fn extend<T: JobElement>(&mut self, h: JobBuffer<T>, v: &[T], c: &mut dyn WorkControl) -> Result<()>;
    fn slice<T: JobElement>(&self, h: JobBuffer<T>) -> Result<&[T]>;
    fn slice_mut<T: JobElement>(&mut self, h: JobBuffer<T>) -> Result<&mut [T]>;
    fn capacity<T: JobElement>(&self, h: JobBuffer<T>) -> Result<usize>;
    fn release<T: JobElement>(&mut self, h: JobBuffer<T>) -> Result<()>;
}
impl Storage for RasterJob {
    fn buffer<T: JobElement>(&mut self, p: RasterPool, n: usize, c: &mut dyn WorkControl) -> Result<JobBuffer<T>> { Self::buffer(self,p,n,c) }
    fn reserve<T: JobElement>(&mut self,h:JobBuffer<T>,n:usize,c:&mut dyn WorkControl)->Result<()> { Self::reserve(self,h,n,c) }
    fn extend<T: JobElement>(&mut self,h:JobBuffer<T>,v:&[T],c:&mut dyn WorkControl)->Result<()> { Self::extend(self,h,v,c) }
    fn slice<T: JobElement>(&self,h:JobBuffer<T>)->Result<&[T]> { Self::slice(self,h) }
    fn slice_mut<T: JobElement>(&mut self,h:JobBuffer<T>)->Result<&mut [T]> { Self::slice_mut(self,h) }
    fn capacity<T: JobElement>(&self,h:JobBuffer<T>)->Result<usize> { Self::capacity(self,h) }
    fn release<T: JobElement>(&mut self,h:JobBuffer<T>)->Result<()> { Self::release(self,h) }
}
impl Storage for CacheTransaction<'_> {
    fn buffer<T: JobElement>(&mut self, p: RasterPool, n: usize, c: &mut dyn WorkControl) -> Result<JobBuffer<T>> { Self::buffer(self,p,n,c) }
    fn reserve<T: JobElement>(&mut self,h:JobBuffer<T>,n:usize,c:&mut dyn WorkControl)->Result<()> { Self::reserve(self,h,n,c) }
    fn extend<T: JobElement>(&mut self,h:JobBuffer<T>,v:&[T],c:&mut dyn WorkControl)->Result<()> { Self::extend(self,h,v,c) }
    fn slice<T: JobElement>(&self,h:JobBuffer<T>)->Result<&[T]> { Self::slice(self,h) }
    fn slice_mut<T: JobElement>(&mut self,h:JobBuffer<T>)->Result<&mut [T]> { Self::slice_mut(self,h) }
    fn capacity<T: JobElement>(&self,h:JobBuffer<T>)->Result<usize> { Self::capacity(self,h) }
    fn release<T: JobElement>(&mut self,h:JobBuffer<T>)->Result<()> { Self::release(self,h) }
}
