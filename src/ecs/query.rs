use crate::ecs::component::ComponentId;

pub type Query = Vec<ComponentId>;

/// Creates a macro to easily define queries to the ECS
#[macro_export]
macro_rules! query{
    // Case where we only want to select one component
    ($ecs:expr, $component:ty) => {
        Vec::from([$ecs.component_id::<$component>().unwrap()])
    };

    // Case where we have more than one component
    ($ecs:expr, $component:ty$(, $components:ty)+) => {
        {
            let mut v = Vec::from([$ecs.component_id::<$component>().unwrap()]);
            v.append(&mut query!($ecs$(, $components)+));
            v
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::ecs::EntityHandler;
    use super::*;

    #[test]
    #[should_panic]
    fn query_macro_panic() {
        let mut ecs = EntityHandler::default();
        query!(ecs, usize);
    }

    #[test]
    fn query_macro_single() {
        let mut ecs = EntityHandler::default();
        let entity = ecs.spawn_entity();
        ecs.bind_component::<usize>(entity, 3);
        let component_id = ecs.component_id::<usize>().unwrap();

        let query: Vec<ComponentId> = query!(ecs, usize);
        assert_eq!(query, vec![component_id])
    }

    #[test]
    fn query_macro_multiple() {
        let mut ecs = EntityHandler::default();
        let entity = ecs.spawn_entity();
        ecs.bind_component::<usize>(entity, 3);
        ecs.bind_component::<f32>(entity, 3.0);
        ecs.bind_component::<(usize, usize)>(entity, (0, 1));
        ecs.bind_component::<Option<bool>>(entity, Some(true));

        let query: Vec<ComponentId> = query!(ecs, f32, Option<bool>);
        assert_eq!(query, vec![1, 3])
    }
}