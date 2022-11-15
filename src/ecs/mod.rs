use std::collections::HashMap;
use crate::ecs::archetype::{ArchetypeId, ArchetypeRegister};
use crate::ecs::component::{ComponentId, ComponentsRegister};
use crate::ecs::entity::{EntityId, EntityInfo};

mod component;
mod archetype;
mod entity;
mod query;
mod components_impl;

pub use components_impl::*;

/// Structure responsible for storing, modifying, and generally managing entities
#[derive(Default)]
pub struct EntityHandler {
    entities: EntityId,
    entity_infos: HashMap<EntityId, EntityInfo>,

    archetypes: ArchetypeRegister,

    components: ComponentsRegister,
    component_archetypes: HashMap<ComponentId, HashMap<ArchetypeId, usize>>
}
impl EntityHandler {
    pub fn spawn_entity(&mut self) -> EntityId {
        let entity = self.new_entity_id();
        let unit_archetype = self.archetypes.unit_archetype_mut();
        let row_index = unit_archetype.push_row(vec!());

        self.entity_infos.insert(entity, EntityInfo {
            archetype_id: unit_archetype.id,
            row_index
        });

        entity
    }
    pub fn delete_entity(&mut self, entity: EntityId) {
        // Note that since the number of entities in the world should never be growing,
        // entity IDs are not reused if the entity is deleted as it is not necessary
        let entity_info = self.entity_infos.get(&entity)
            .unwrap_or_else(|| panic!("tried using an invalid entity id"));
        self.archetypes.get_mut(entity_info.archetype_id).remove_row(entity_info.row_index);
        self.entity_infos.remove(&entity);
    }

    /// Checks whether an entity has a component of type ComponentType
    pub fn has_component<ComponentType: 'static>(&self, entity: EntityId) -> bool {
        let entity_info = self.entity_infos.get(&entity)
            .unwrap_or_else(|| panic!("tried using an invalid entity id"));
        let archetype_id = entity_info.archetype_id;

        if let Some(id) = self.components.try_component_id::<ComponentType>() {
            self.archetypes.get(archetype_id).has_component(id)
        } else {
            false
        }
    }

    /// Returns a reference to the given component of an entity, if it exists
    pub fn component<ComponentType: 'static>(&self, entity: EntityId) -> Option<&ComponentType> {
        self.components.try_component_id::<ComponentType>().map(|component_id| {
            let entity_info = self.entity_infos.get(&entity)
                .unwrap_or_else(|| panic!("tried using an invalid entity id"));

            let archetype_id = entity_info.archetype_id;
            let archetype = self.archetypes.get(archetype_id);

            let column_index = self.component_archetypes
                .get(&component_id).unwrap()
                .get(&archetype_id).unwrap();
            if self.has_component::<ComponentType>(entity) {
                Some(archetype.component::<ComponentType>(*column_index, entity_info.row_index))
            } else {
                None
            }
        }).flatten()
    }

    /// Returns a mutable reference to the given component of an entity, if it exists
    pub fn component_mut<ComponentType: 'static>(&mut self, entity: EntityId) -> Option<&mut ComponentType> {
        self.components.try_component_id::<ComponentType>().map(|component_id| {
            let entity_info = self.entity_infos.get(&entity)
                .unwrap_or_else(|| panic!("tried using an invalid entity id"));

            let archetype_id = entity_info.archetype_id;
            if self.archetypes.get(archetype_id).has_component(component_id) {
                let archetype = self.archetypes.get_mut(archetype_id);

                let column_index = self.component_archetypes
                    .get(&component_id).unwrap()
                    .get(&archetype_id).unwrap();
                Some(archetype.component_mut::<ComponentType>(*column_index, entity_info.row_index))
            } else {
                None
            }
        }).flatten()
    }

    /// Binds a new component to an entity
    pub fn bind_component<ComponentType: 'static>(&mut self, entity: EntityId, component: ComponentType) {
        // If the component already exists for our entity, we just change its value
        if self.has_component::<ComponentType>(entity) {
            panic!("Tried binding a component that was already bound, try changing its value using component_mut() instead");
        }

        let entity_info = self.entity_infos.get_mut(&entity)
            .unwrap_or_else(|| panic!("tried using an invalid entity id"));
        let component_id = self.components.component_id::<ComponentType>();
        let src_archetype_id = entity_info.archetype_id;

        // Removes our entity's components from the old archetype's storage, and adds the new component
        // to it
        let mut components_row = self.archetypes.get_mut(src_archetype_id).remove_row(entity_info.row_index);
        components_row.push(Box::new(component));

        // Fetches, or creates, our extended archetype, and moves our entity to said archetype
        let new_archetype = self.archetypes.extend_archetype(src_archetype_id, component_id);
        entity_info.archetype_id = new_archetype.id;
        entity_info.row_index = new_archetype.push_row(components_row);

        // Register our component index
        if let Some(component_archetype) = self.component_archetypes.get_mut(&component_id) {
            // The component already exists
            if !component_archetype.contains_key(&new_archetype.id) {
                // We've created a new archetype and need to index our component to it
                let column_index = new_archetype.signature.len() - 1;
                component_archetype.insert(new_archetype.id, column_index);
            }
        } else {
            // Our component type is new
            let column_index = new_archetype.signature.len() - 1;
            self.component_archetypes.insert(component_id, HashMap::from([(new_archetype.id, column_index)]));
        }
    }

    fn new_entity_id(&mut self) -> EntityId {
        self.entities += 1;
        self.entities - 1
    }
}


#[cfg(test)]
mod tests {
    use std::any::Any;
    use super::*;

    #[test]
    fn add_entities() {
        let mut entity_handler = EntityHandler::default();
        assert_eq!(entity_handler.spawn_entity(), 0);
        assert_eq!(entity_handler.spawn_entity(), 1);
    }

    #[test]
    fn entity_removal() {
        let mut entity_handler = EntityHandler::default();
        let entity = entity_handler.spawn_entity();
        entity_handler.delete_entity(entity);
        assert_eq!(entity_handler.spawn_entity(), 1);
    }

    #[test]
    fn bind_component() {
        let mut entity_handler = EntityHandler::default();
        let entity = entity_handler.spawn_entity();
        entity_handler.bind_component(entity, (3, Some(2)))
    }

    #[test]
    fn get_component() {
        let mut entity_handler = EntityHandler::default();
        let entity = entity_handler.spawn_entity();
        entity_handler.bind_component(entity, String::from("aled"));
        assert_eq!(entity_handler.component::<String>(entity), Some(&String::from("aled")));
        assert_eq!(entity_handler.component::<usize>(entity), None)
    }

    #[test]
    fn get_component_mut() {
        let mut entity_handler = EntityHandler::default();
        let entity = entity_handler.spawn_entity();
        {
            let vec: Vec<u8> = vec![0, 1];
            entity_handler.bind_component(entity, vec);
            let component_mut = entity_handler.component_mut::<Vec<u8>>(entity).unwrap();
            component_mut.push(2);
        }
        assert_eq!(entity_handler.component::<Vec<u8>>(entity), Some(&vec![0, 1, 2]))
    }

    #[test]
    #[should_panic]
    fn rebinding_panics() {
        let mut entity_handler = EntityHandler::default();
        let entity = entity_handler.spawn_entity();
        entity_handler.bind_component(entity, String::from("aled"));
        entity_handler.bind_component(entity, String::from("aaaaa"));
    }
}