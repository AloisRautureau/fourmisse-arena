use crate::ecs::archetype::ArchetypeId;

pub type EntityId = usize;

#[derive(Default, Debug)]
pub struct EntityInfo {
    pub archetype_id: ArchetypeId,
    pub row_index: usize,
}
