use std::any::{Any, TypeId};
use std::collections::HashMap;

pub type ComponentId = usize;

#[derive(Default)]
pub struct ComponentsRegister {
    ids: HashMap<TypeId, ComponentId>
}
impl ComponentsRegister {
    /// Returns the id of the given component, creating it in the process if it does not yet exist
    pub fn component_id<ComponentType: 'static>(&mut self) -> ComponentId {
        if self.ids.contains_key(&TypeId::of::<ComponentType>()) {
            *self.ids.get(&TypeId::of::<ComponentType>()).unwrap()
        } else {
            self.register_component::<ComponentType>()
        }
    }

    /// Same as component_id(), except it does not create a new id if the component
    /// is not registered. Useful if the component register should be immutable
    pub fn try_component_id<ComponentType: 'static>(&self) -> Option<ComponentId> {
        self.ids.get(&TypeId::of::<ComponentType>()).copied()
    }

    /// Registers a new component, returning its ID
    /// This should be kept private, as the component creation logic should only be accessible
    /// by the ComponentRegister
    fn register_component<ComponentType: 'static>(&mut self) -> ComponentId {
        let id = self.ids.len();
        self.ids.insert(TypeId::of::<ComponentType>(), id);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_id_creates_new() {
        let mut component_register = ComponentsRegister::default();
        assert_eq!(component_register.component_id::<usize>(), 0);
        assert_eq!(component_register.component_id::<(f32, usize)>(), 1);
    }

    #[test]
    fn component_id_returns_old() {
        let mut component_register = ComponentsRegister::default();
        assert_eq!(component_register.component_id::<usize>(), 0);
        assert_eq!(component_register.component_id::<usize>(), 0);
    }

    fn try_component_id() {
        let mut component_register = ComponentsRegister::default();
        assert_eq!(component_register.try_component_id::<usize>(), None);
        let usize_id = component_register.component_id::<usize>();
        assert_eq!(component_register.try_component_id::<usize>(), Some(usize_id));
    }
}