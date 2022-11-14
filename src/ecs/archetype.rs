use std::any::{Any, TypeId};
use std::collections::HashMap;
use crate::ecs::component::ComponentId;

pub type ArchetypeId = usize;
pub type Signature = Vec<ComponentId>;
/// Applies a "xor" operation on a signature
fn xor_signature(signature: &Signature, component_id: ComponentId) -> Signature {
    let mut new_signature = signature.clone();
    if let Some(i) = new_signature.iter().position(|e| *e == component_id) {
        // The component exists in the signature
        new_signature.remove(i);
    } else if let Some(i) = new_signature.iter().position(|e| *e > component_id) {
        // The component should be placed at index i to keep the signature sorted
        new_signature.insert(i, component_id);
    } else {
        // The component should be placed at the end of the signature
        new_signature.push(component_id);
    }
    new_signature
}

/// Stores every archetype that was ever created
pub struct ArchetypeRegister {
    archetypes: Vec<Box<Archetype>>
}
impl Default for ArchetypeRegister {
    fn default() -> Self {
        // The base archetype needs to be created, it holds every entity with no components
        ArchetypeRegister {
            archetypes: vec![Box::new(Archetype {
                id: 0,
                signature: vec!(),
                .. Default::default()
            })]
        }
    }
}
impl ArchetypeRegister {
    pub fn get(&self, archetype_id: ArchetypeId) -> &Box<Archetype> {
        &self.archetypes[archetype_id]
    }
    pub fn get_mut(&mut self, archetype_id: ArchetypeId) -> &mut Box<Archetype> {
        &mut self.archetypes[archetype_id]
    }

    /// Returns the unit archetype
    pub fn unit_archetype(&self) -> &Box<Archetype> {
        self.get(0)
    }
    pub fn unit_archetype_mut(&mut self) -> &mut Box<Archetype> {
        self.get_mut(0)
    }

    /// Registers a new archetype created by adding/removing a component from another, known, archetype
    /// This returns a mutable reference for convenience, as the EntityHandler needs to perform a
    /// few modifications on the resulting archetype afterwards
    pub fn extend_archetype(&mut self, src_archetype_id: ArchetypeId, component_id: ComponentId) -> &mut Box<Archetype> {
        let next_archetype_id = {
            let src_archetype = self.get(src_archetype_id);
            src_archetype.next_archetype(component_id)
        };

        let dest_archetype_id = {
            // Check if the extended archetype is already registered
            if let Some(dest_archetype_id) = next_archetype_id {
                dest_archetype_id
            } else {
                // If not, we must create, register, and add the new archetype to our archetype graph
                let id = self.archetypes.len();
                let dest_archetype = {
                    let mut src_archetype = self.get_mut(src_archetype_id);
                    let signature = xor_signature(&src_archetype.signature, component_id);
                    let mut dest_archetype = Archetype {
                        id,
                        signature,
                        ..Default::default()
                    };
                    // The new archetype should return the source one, and vice versa
                    dest_archetype.add_edge(component_id, src_archetype_id);
                    src_archetype.add_edge(component_id, id);
                    dest_archetype
                };

                self.archetypes.push(Box::new(dest_archetype));
                id
            }
        };

        self.get_mut(dest_archetype_id)
    }

    /// Looks up an archetype matching the given signature
    pub fn lookup(&self, signature: &Signature) -> Option<&Box<Archetype>> {
        self.archetype_id_from_signature(signature).map(|id| self.get(id))
    }
    pub fn lookup_mut(&mut self, signature: &Signature) -> Option<&mut Box<Archetype>> {
        self.archetype_id_from_signature(signature).map(|id| self.get_mut(id))
    }
    fn archetype_id_from_signature(&self, signature: &Signature) -> Option<usize> {
        let mut current = 0;
        // Since the archetypes are represented as a graph, we can simply add up every component,
        // following the edges of the graph to find our archetype
        for component_id in signature {
            if let Some(archetype_id) = self.get(current).next_archetype(*component_id) {
                current = archetype_id
            } else {
                return None
            }
        }
        Some(current)
    }

    /// Returns a reference to any archetype matching the query
    pub fn query(&self, query: ()) -> Vec<&Archetype> {
        todo!()
    }
}

/// This structure defines signatures for entities, as a set of components (additive typing)
/// It also stores entities of said "type", and references to extended/reduced archetypes
/// as to form a graph with archetypes as nodes, with an edge by added/removed component
#[derive(Default)]
pub struct Archetype {
    pub id: ArchetypeId,
    pub signature: Signature,

    pub components: Vec<Vec<Box<dyn Any>>>,
    edges: HashMap<ComponentId, ArchetypeId>,
}
impl Archetype {
    /// Returns the index of a new row in the components table
    pub fn push_row(&mut self, row: Vec<Box<dyn Any>>) -> usize {
        self.components.push(row);
        self.components.len() - 1
    }
    /// Removes the row at the given index, returning the removed row
    pub fn remove_row(&mut self, row_index: usize) -> Vec<Box<dyn Any>> {
        self.components.remove(row_index)
    }

    /// Checks if the archetype is composed of a certain component type
    pub fn has_component(&self, component_id: ComponentId) -> bool {
        self.signature.contains(&component_id)
    }

    /// Returns the archetype following the edge of the archetype graph
    pub fn next_archetype(&self, component_id: ComponentId) -> Option<ArchetypeId> {
        self.edges.get(&component_id).copied()
    }

    /// Adds a new edge to the archetype graph
    pub fn add_edge(&mut self, component_id: ComponentId,  dest_archetype: ArchetypeId) {
        self.edges.insert(component_id, dest_archetype);
    }

    /// Returns a reference to a component stored at the given column and row, casting it to a certain type
    /// Note that this function panics if ComponentType does not match the actual type of the stored component
    pub fn component<ComponentType: 'static>(&self, column: usize, row: usize) -> &ComponentType {
        self.components[column][row].downcast_ref::<ComponentType>().unwrap()
    }
    pub fn component_mut<ComponentType: 'static>(&mut self, column: usize, row: usize) -> &mut ComponentType {
        self.components[column][row].downcast_mut::<ComponentType>().unwrap()
    }

    /// Sets the value of the given column/row
    pub fn set_component<ComponentType: 'static>(&mut self, component: ComponentType, column: usize, row: usize) {
        self.components[column][row] = Box::new(component);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_signature() {
        let signature = vec![0, 2, 3];
        assert_eq!(super::xor_signature(&signature, 0), vec![2, 3]);
        assert_eq!(super::xor_signature(&signature, 1), vec![0, 1, 2, 3]);
        assert_eq!(super::xor_signature(&signature, 5), vec![0, 2, 3, 5]);
    }

    #[test]
    fn unit_archetype_on_init() {
        let archetype_register = ArchetypeRegister::default();
        assert_eq!(archetype_register.unit_archetype().signature, vec!())
    }

    #[test]
    fn extend_archetype() {
        let mut archetype_register = ArchetypeRegister::default();
        let a1 = archetype_register.extend_archetype(0, 0).id;
        let a2 = archetype_register.extend_archetype(a1, 2).id;

        assert_eq!(archetype_register.get(a1).next_archetype(2), Some(a2));
        assert_eq!(archetype_register.get(a1).next_archetype(0), Some(0));
        assert_eq!(archetype_register.get(a2).next_archetype(0), None)
    }

    #[test]
    fn lookup() {
        let mut archetype_register = ArchetypeRegister::default();
        archetype_register.extend_archetype(0, 1232);

        assert!(archetype_register.lookup(&vec!()).is_some());
        assert!(archetype_register.lookup(&vec![1232]).is_some());
        assert!(archetype_register.lookup(&vec![1, 2]).is_none());
    }

    // Testing for the Archetype struct is mostly done in integration (see EntityHandler tests) as
    // most functions only use standard library methods
}