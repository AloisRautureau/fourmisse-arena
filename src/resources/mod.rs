mod instructions_loader;
pub use instructions_loader::InstructionsLoader;

use std::any::Any;
use std::collections::HashMap;

/// Implemented by structures that can load resources of a given type from disk
pub trait ResourceLoader {
    type Item;
    // self is made mutable here to allow more flexibility in the way we load stuff
    fn load(&mut self) -> Self::Item;
}

pub type ResourceId = usize;

/// Used to store resources that are loaded from disk at runtime, like models,
/// textures or instruction sets
/// Those can then be accessed by name, using a string as an identifier
#[derive(Default)]
pub struct ResourceHandler {
    resources: HashMap<ResourceId, Box<dyn Any>>
}
impl ResourceHandler {
    /// Loads up a new resource of the given type, setting id as its key
    pub fn load<ResourceType: 'static>(&mut self, loader: &mut dyn ResourceLoader<Item=ResourceType>) -> ResourceId {
        let id = self.resources.len();
        let resource = loader.load();
        self.resources.insert(id, Box::new(resource));
        id
    }

    /// Getter functions
    /// Note that if trying to fetch a resource using the wrong type or an invalid id,
    /// this function will panic
    pub fn get<ResourceType: 'static>(&self, resource: ResourceId) -> &ResourceType {
        self.resources.get(&resource)
            .unwrap_or_else(|| panic!("Tried using an invalid resource id"))
            .downcast_ref::<ResourceType>()
            .unwrap_or_else(|| panic!("Tried fetching a resource using an invalid type"))
    }
    pub fn get_mut<ResourceType: 'static>(&mut self, resource: ResourceId) -> &mut ResourceType {
        self.resources.get_mut(&resource)
            .unwrap_or_else(|| panic!("Tried using an invalid resource id"))
            .downcast_mut::<ResourceType>()
            .unwrap_or_else(|| panic!("Tried fetching a resource using an invalid type"))
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::BorrowMut;
    use super::*;
    
    // Dummy implementation for tests
    struct UsizeLoader {}
    impl ResourceLoader for UsizeLoader {
        type Item = usize;
        fn load(&mut self) -> Self::Item {
            1
        }
    }

    #[test]
    fn load_id() {
        let mut resource_handler = ResourceHandler::default();
        let id = resource_handler.load(&mut UsizeLoader {});
        assert_eq!(id, 0)
    }

    #[test]
    fn get_valid() {
        let mut resource_handler = ResourceHandler::default();
        let id = resource_handler.load(&mut UsizeLoader {});
        assert_eq!(resource_handler.get::<usize>(id), &1)
    }

    #[test]
    #[should_panic]
    fn get_invalid_id() {
        let mut resource_handler = ResourceHandler::default();
        let _ = resource_handler.load(&mut UsizeLoader {});
        resource_handler.get::<usize>(12);
    }
    
    #[test]
    #[should_panic]
    fn get_invalid_type() {
        let mut resource_handler = ResourceHandler::default();
        let id = resource_handler.load(&mut UsizeLoader {});
        resource_handler.get::<String>(id);
    }

    #[test]
    fn get_mut() {
        let mut resource_handler = ResourceHandler::default();
        let id = resource_handler.load(&mut UsizeLoader {});
        let usize_mut = resource_handler.get_mut::<usize>(id);
        *usize_mut = 2;

        assert_eq!(resource_handler.get::<usize>(id), &2);
    }
}