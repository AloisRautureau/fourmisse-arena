use std::fmt::{Debug, Formatter};
use rustc_hash::{FxHashMap, FxHashSet};
use crate::ecs::archetype::{ArchetypeId, ArchetypeRegister};
use crate::ecs::component::{ComponentId, ComponentsRegister};
use crate::ecs::entity::EntityInfo;

mod component;
mod archetype;
mod entity;
mod query;
mod components_impl;

pub use entity::EntityId;
pub use components_impl::*;
use crate::ecs::query::Query;

/// Structure responsible for storing, modifying, and generally managing entities
#[derive(Default)]
pub struct EntityHandler {
    entities: EntityId,
    entity_infos: Vec<EntityInfo>,

    archetypes: ArchetypeRegister,

    components: ComponentsRegister,
    component_archetypes: Vec<FxHashMap<ArchetypeId, usize>>
}
impl EntityHandler {
    pub fn spawn_entity(&mut self) -> EntityId {
        let entity = self.new_entity_id();
        let unit_archetype = self.archetypes.unit_archetype_mut();
        let row_index = unit_archetype.push_row(entity, vec!());

        self.entity_infos.push(EntityInfo {
            archetype_id: unit_archetype.id,
            row_index
        });

        entity
    }
    pub fn delete_entity(&mut self, entity: EntityId) {
        // Note that since the number of entities in the world should never be growing,
        // entity IDs are not reused if the entity is deleted as it is not necessary
        let entity_info = &self.entity_infos[entity];
        self.archetypes.get_mut(entity_info.archetype_id).remove_row(entity_info.row_index);
        self.entity_infos.remove(entity);
    }

    /// Checks whether an entity has a component of type ComponentType
    pub fn has_component<ComponentType: 'static>(&self, entity: EntityId) -> bool {
        let entity_info = &self.entity_infos[entity];
        let archetype_id = entity_info.archetype_id;

        if let Some(id) = self.components.try_component_id::<ComponentType>() {
            self.component_archetypes[id].contains_key(&archetype_id)
        } else {
            false
        }
    }

    /// Returns a reference to the given component of an entity, if it exists
    pub fn component<ComponentType: 'static>(&self, entity: EntityId) -> Option<&ComponentType> {
        if let Some(component_id) = self.components.try_component_id::<ComponentType>() {
            let entity_info = &self.entity_infos[entity];
            let archetype_id = entity_info.archetype_id;

            if self.has_component::<ComponentType>(entity) {
                self.get_component_column(component_id, &archetype_id)
                    .map(|column_index|
                        self.archetypes.get(archetype_id).component::<ComponentType>(column_index, entity_info.row_index)
                    )
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Returns a mutable reference to the given component of an entity, if it exists
    pub fn component_mut<ComponentType: 'static>(&mut self, entity: EntityId) -> Option<&mut ComponentType> {
        if let Some(component_id) = self.components.try_component_id::<ComponentType>() {
            let entity_info = &self.entity_infos[entity];
            let archetype_id = entity_info.archetype_id;

            if self.has_component::<ComponentType>(entity) {
                self.get_component_column(component_id, &archetype_id)
                    .map(|column_index|
                        self.archetypes.get_mut(archetype_id).component_mut::<ComponentType>(column_index, entity_info.row_index)
                    )
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Binds a new component to an entity
    pub fn bind_component<ComponentType: 'static>(&mut self, entity: EntityId, component: ComponentType) {
        // If the component already exists for our entity, we'd need to change its value instead
        if self.has_component::<ComponentType>(entity) {
            panic!("Tried binding a component that was already bound, try changing its value using component_mut() instead");
        }

        let entity_info = &self.entity_infos[entity];
        let src_archetype_id = entity_info.archetype_id;
        let component_id = self.components.component_id::<ComponentType>();
        let new_archetype = self.archetypes.extend_archetype(src_archetype_id, component_id);

        // Removes our entity's components from the old archetype's storage, and adds the new component
        // to it
        let component_column = self.archetypes.get(new_archetype).signature.iter().position(|c| *c == component_id).unwrap();
        let (mut components_row, entities_to_update) = self.archetypes.get_mut(src_archetype_id).remove_row(entity_info.row_index);
        components_row.insert(component_column, Box::new(component));
        for entity in entities_to_update {
            self.entity_infos[*entity].row_index -= 1
        }

        // Fetches, or creates, our extended archetype, and moves our entity to said archetype
        let mut entity_info = &mut self.entity_infos[entity];
        entity_info.archetype_id = new_archetype;
        entity_info.row_index = self.archetypes.get_mut(new_archetype).push_row(entity, components_row);

        // Register our component index
        self.component_archetypes.push(FxHashMap::default());
        for (i, component) in self.archetypes.get(new_archetype).signature.iter().enumerate() {
            self.component_archetypes[*component].insert(new_archetype, i);
        }
    }

    pub fn component_id<ComponentType: 'static>(&self) -> Option<ComponentId> {
        self.components.try_component_id::<ComponentType>()
    }

    /// Given a query (list of components), returns all entities which satisfy said query
    pub fn query(&self, query: &Query) -> impl Iterator<Item = EntityId> {
        let mut entities = Vec::new();
        for archetype_id in self.query_archetypes(query) {
            entities.append(&mut self.archetypes.get(*archetype_id).entity_ids.clone())
        }

        entities.into_iter()
    }

    /// Given a query (list of components), returns all matching archetypes
    pub fn query_archetypes(&self, query: &Query) -> impl Iterator<Item = &ArchetypeId> {
        let mut query_iterator = query.iter();
        let mut result = self.component_archetypes(*query_iterator.next().unwrap());

        // Check matching archetypes
        for component in query_iterator {
            // We filter archetypes that do not match the new component in our result
            result = result.intersection(&self.component_archetypes(*component)).copied().collect();
        }

        result.into_iter()
    }

    fn component_archetypes(&self, component_id: ComponentId) -> FxHashSet<&ArchetypeId> {
        if let Some(map) = self.component_archetypes.get(component_id) {
            map.keys().collect()
        } else {
            FxHashSet::default()
        }
    }

    fn new_entity_id(&mut self) -> EntityId {
        self.entities += 1;
        self.entities - 1
    }

    fn get_component_column(&self, component_id: ComponentId, archetype_id: &ArchetypeId) -> Option<usize> {
        self.component_archetypes.get(component_id).map(|r| r.get(archetype_id)).flatten().copied()
    }
}
impl Debug for EntityHandler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "entities: {}", self.entities)?;
        writeln!(f, "entity infos:\n{:?}", self.entity_infos)?;
        writeln!(f, "{:?}", self.archetypes)?;
        write!(f, "component infos:\n{:?}", self.component_archetypes)

    }
}


#[cfg(test)]
mod tests {
    use std::vec;
    use crate::query;
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

    #[test]
    fn query() {
        let mut entity_handler = EntityHandler::default();
        let mut entities = vec!();
        for _ in 0..25 {
            entities.push(entity_handler.spawn_entity());
        }

        let mut usize_entities = vec!();
        let mut f32_entities = vec!();
        let mut f32_usize_entities = vec!();
        for entity in entities.clone() {
            if entity % 2 == 0 {
                entity_handler.bind_component::<usize>(entity, 0);
                usize_entities.push(entity);
            } else if entity % 5 == 0 {
                entity_handler.bind_component::<f32>(entity, 1.2);
                entity_handler.bind_component::<usize>(entity, 1);
                f32_entities.push(entity);
                usize_entities.push(entity);
                f32_usize_entities.push(entity);
            } else {
                entity_handler.bind_component::<f32>(entity, 1.2);
                f32_entities.push(entity);
            }
        }

        let mut usize_query_result: Vec<EntityId> = entity_handler.query(&query!(entity_handler, usize)).collect();
        usize_query_result.sort();
        let mut f32_query_result: Vec<EntityId> = entity_handler.query(&query!(entity_handler, f32)).collect();
        f32_query_result.sort();
        let mut both_query_result: Vec<EntityId> = entity_handler.query(&query!(entity_handler, f32, usize)).collect();
        both_query_result.sort();

        assert_eq!(usize_query_result, usize_entities);
        assert_eq!(f32_query_result, f32_entities);
        assert_eq!(both_query_result, f32_usize_entities);
    }
}