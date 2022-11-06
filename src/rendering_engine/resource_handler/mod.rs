use std::marker;

mod model_manager;
pub use model_manager::ModelVec;

#[derive(Default, Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct ResourceHandle(pub usize);
impl ResourceHandle {
    const STANDARD_INDEX_MASK: usize = (1 << 16) - 1;

    // Checks whether the handle is currently valid
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
    // Returns the handle's index
    pub fn index(&self) -> usize {
        self.0 & Self::STANDARD_INDEX_MASK
    }
    // Returns the resource magic number
    pub fn magic(&self) -> usize {
        (self.0 & !Self::STANDARD_INDEX_MASK) >> 16
    }
}

// A ResourceVec is given ownership of every loaded resource of a given type, that way we avoid reloading
// a single resource multiple times (e.g a model that will be reused often)
pub trait ResourceVec {
    fn load(&mut self, path: &str) -> ResourceHandle;
}

// The ResourceHandler contains every single resource we currently can load from disk
#[derive(Default)]
pub struct ResourceHandler {
    pub models: ModelVec,
}
