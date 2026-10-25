use bevy_ecs::component::Component;
use bevy_ecs::entity::{Entity, EntityHashMap, EntityHashSet};
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::world::{DeferredWorld, World};

use super::{CharacterRole, RelationType};

// vvv many-to-many relation structure ripped from https://codeberg.org/voidentente/bevy_n2m/src/branch/main/src/lib.rs

pub trait RelationshipCollection: Send + Sync + Default + 'static {
    fn increment(&mut self, complementary_entity: Entity, relationship_entity: Entity);
    fn decrement(&mut self, complementary_entity: Entity, relationship_entity: Entity);
    fn iter(&self) -> impl Iterator<Item = (Entity, impl Iterator<Item = Entity>)>;
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool { self.len() == 0 }
}

impl RelationshipCollection for EntityHashMap<BidrectionalRelations> {
    fn increment(&mut self, complementary_entity: Entity, relationship_entity: Entity) {
        self.entry(complementary_entity).or_default().insert(relationship_entity);
    }
    fn decrement(&mut self, complementary_entity: Entity, relationship_entity: Entity) {
        let Some(set) = self.get_mut(&complementary_entity) else { return; };
        set.remove(relationship_entity);
        if set.is_empty() {
            self.remove(&complementary_entity);
        }
    }
    fn iter(&self) -> impl Iterator<Item = (Entity, impl Iterator<Item = Entity>)> {
        (**self).iter()
            .map(|(e, rel)| (*e, rel.0.iter().filter_map(|e| *e)))
    }
    fn len(&self) -> usize {
        (**self).len()
    }

}

impl RelationshipCollection for EntityHashMap<EntityHashSet> {
    fn increment(&mut self, complementary_entity: Entity, relationship_entity: Entity) {
        self.entry(complementary_entity).or_default().insert(relationship_entity);
    }
    fn decrement(&mut self, complementary_entity: Entity, relationship_entity: Entity) {
        let Some(set) = self.get_mut(&complementary_entity) else { return; };
        set.remove(&relationship_entity);
        if set.is_empty() {
            self.remove(&complementary_entity);
        }
    }
    fn iter(&self) -> impl Iterator<Item = (Entity, impl Iterator<Item = Entity>)> {
        (**self).iter()
            .map(|(e, coll)| (*e, coll.iter().copied()))
    }
    fn len(&self) -> usize {
        (**self).len()
    }
}

// all media relations are two-way relations (others are one-way but can still be represented here)
// ex. m1 -> m2 (Sequel), m2 -> m1 (Prequel)
#[derive(Debug, Default)]
pub struct BidrectionalRelations([Option<Entity>; 2]);

impl BidrectionalRelations {
    fn insert(&mut self, ent: Entity) {
        let Self([slot_1, slot_2]) = self;
        match (*slot_1, *slot_2) {
            (None, None | Some(_)) => { let _ = slot_1.insert(ent); }
            (Some(_), None) => { let _ = slot_2.insert(ent); }
            (Some(_), Some(_)) => {},
        }
    }
    fn remove(&mut self, ent: Entity) {
        let Self([slot_1, slot_2]) = self;
        if slot_1.is_some_and(|s| s == ent) {
            slot_1.take();
        } else if slot_2.is_some_and(|s| s == ent) {
            slot_2.take();
        } else {}
    }
    fn is_empty(&self) -> bool {
        self.0 == [None, None]
    }
}

// relation entity on target + source entity counting the complementary (opposite) entities
#[derive(Debug, Component)]
#[component(on_remove = Self::on_remove)]
pub struct RelationCollectionTarget {
    // must be a hash set to support every relationship type (bidirectional + many-to-many)
    collection: EntityHashMap<EntityHashSet>,
}

impl RelationCollectionTarget {
    pub fn new() -> Self {
        Self { collection: EntityHashMap::default() }
    }

    // vvv see `bevy_n2m`

    // complementary entity -> the other entity in the relationship (i.e. the one on the other side -> ex. MediaRelation)
    fn increment_or(mut world: DeferredWorld<'_>, ent: Entity, comp_ent: Entity, rel_ent: Entity) {
        if let Some(mut target) = world.entity_mut(ent).get_mut::<Self>() {
            target.collection.increment(comp_ent, rel_ent);
        } else {
            world.commands().queue(move |world: &mut World| {
                let Ok(mut ent_mut) = world.get_entity_mut(ent) else { return; };
                if let Some(mut target) = ent_mut.get_mut::<Self>() {
                    target.collection.increment(comp_ent, rel_ent);
                } else {
                    let mut target = Self::new();
                    target.collection.increment(comp_ent, rel_ent);
                    ent_mut.insert(target);
                }
            });
        }
    }

    fn decrement_or(mut world: DeferredWorld<'_>, ent: Entity, comp_ent: Entity, rel_ent: Entity) {
        let Ok(mut ent_mut) = world.get_entity_mut(ent) else { return; };
        let Some(mut target) = ent_mut.get_mut::<Self>() else { return; };
        target.collection.decrement(comp_ent, rel_ent);

        if target.collection.is_empty() {
            world.commands().queue(move |world: &mut World| {
                let Ok(mut ent_mut) = world.get_entity_mut(ent) else { return; };
                let Some(target) = ent_mut.get::<Self>() else { return; };

                if target.collection.is_empty() { ent_mut.remove::<Self>(); }
            });
        }
    }

    fn on_remove(mut world: DeferredWorld<'_>, ctx: HookContext) {
        let (ents, mut cmds) = world.entities_and_commands();
        let Ok(ent_ref) = ents.get(ctx.entity) else { return; };
        let Some(target) = ent_ref.get::<Self>() else { return; };

        for (_, rel_ent) in target.collection.iter() {
            for ent in rel_ent {
                cmds.entity(ent).despawn();
            }
        }
    }
}

// relationship entity (independent from source and target entities)
#[derive(Component)]
#[component(immutable, on_insert = Self::on_insert, on_remove = Self::on_remove)]
pub struct MediaRelation {
    // how necessary are media types here? can i determine the media type from the entity alone, as in reference a component that determines the media type?
    source: Entity,
    target: Entity,
    rel_type: RelationType,
}

impl MediaRelation {
    pub fn new(source: Entity, target: Entity, rel_type: RelationType) -> Self {
        Self { source, target, rel_type }
    }
    pub fn relation_type(&self) -> &RelationType { &self.rel_type }

    fn on_insert(mut world: DeferredWorld<'_>, ctx: HookContext) {
        let &Self { source, target, .. } = world
            .entity(ctx.entity)
            .get::<Self>()
            .expect("no instance of MediaRelation on this entity");

        RelationCollectionTarget::increment_or(world.reborrow(), source, target, ctx.entity);
        RelationCollectionTarget::increment_or(world.reborrow(), target, source, ctx.entity);
    }

    fn on_remove(mut world: DeferredWorld<'_>, ctx: HookContext) {
        let &Self { source, target, .. } = world
            .entity(ctx.entity)
            .get::<Self>()
            .expect("no instance of MediaRelation on this entity");

        RelationCollectionTarget::decrement_or(world.reborrow(), source, target, ctx.entity);
        RelationCollectionTarget::decrement_or(world.reborrow(), target, source, ctx.entity);
    }
}

#[derive(Component)]
#[component(immutable, on_insert = Self::on_insert, on_remove = Self::on_remove)]
pub struct MediaCharacterRelation {
    media: Entity,
    character: Entity,
    role: CharacterRole,
}

impl MediaCharacterRelation {
    pub fn new(media: Entity, character: Entity, role: CharacterRole) -> Self {
        Self { media, character, role }
    }
    pub fn character_role(&self) -> &CharacterRole { &self.role }

    fn on_insert(mut world: DeferredWorld<'_>, ctx: HookContext) {
        let &Self { media, character, .. } = world
            .entity(ctx.entity)
            .get::<Self>()
            .expect("no instance of MediaRelation on this entity");

        RelationCollectionTarget::increment_or(world.reborrow(), media, character, ctx.entity);
        RelationCollectionTarget::increment_or(world.reborrow(), character, media, ctx.entity);
    }

    fn on_remove(mut world: DeferredWorld<'_>, ctx: HookContext) {
        let &Self { media, character, .. } = world
            .entity(ctx.entity)
            .get::<Self>()
            .expect("no instance of MediaRelation on this entity");

        RelationCollectionTarget::decrement_or(world.reborrow(), media, character, ctx.entity);
        RelationCollectionTarget::decrement_or(world.reborrow(), character, media, ctx.entity);
    }
}

#[derive(Component)]
#[component(immutable, on_insert = Self::on_insert, on_remove = Self::on_remove)]
pub struct MediaStaffRelation {
    media: Entity,
    staff: Entity,
    role: String,
}

impl MediaStaffRelation {
    pub fn new(media: Entity, staff: Entity, role: String) -> Self {
        Self { media, staff, role }
    }
    pub fn staff_role(&self) -> &str { &self.role }

    fn on_insert(mut world: DeferredWorld<'_>, ctx: HookContext) {
        let &Self { media, staff, .. } = world
            .entity(ctx.entity)
            .get::<Self>()
            .expect("no instance of MediaRelation on this entity");

        // the hash set here is necessary since a single staff member can have multiple roles on a given series
        RelationCollectionTarget::increment_or(world.reborrow(), media, staff, ctx.entity);
        RelationCollectionTarget::increment_or(world.reborrow(), staff, media, ctx.entity);
    }

    fn on_remove(mut world: DeferredWorld<'_>, ctx: HookContext) {
        let &Self { media, staff, .. } = world
            .entity(ctx.entity)
            .get::<Self>()
            .expect("no instance of MediaRelation on this entity");

        RelationCollectionTarget::decrement_or(world.reborrow(), media, staff, ctx.entity);
        RelationCollectionTarget::decrement_or(world.reborrow(), staff, media, ctx.entity);
    }
}

#[derive(Component)]
#[component(immutable, on_insert = Self::on_insert, on_remove = Self::on_remove)]
pub struct MediaStudioRelation {
    media: Entity,
    studio: Entity,
    is_main: bool,
}

impl MediaStudioRelation {
    pub fn new(media: Entity, studio: Entity, is_main: bool) -> Self {
        Self { media, studio, is_main }
    }
    pub fn is_main(&self) -> bool { self.is_main }

    fn on_insert(mut world: DeferredWorld<'_>, ctx: HookContext) {
        let &Self { media, studio, .. } = world
            .entity(ctx.entity)
            .get::<Self>()
            .expect("no instance of MediaRelation on this entity");

        RelationCollectionTarget::increment_or(world.reborrow(), media, studio, ctx.entity);
        RelationCollectionTarget::increment_or(world.reborrow(), studio, media, ctx.entity);
    }

    fn on_remove(mut world: DeferredWorld<'_>, ctx: HookContext) {
        let &Self { media, studio, .. } = world
            .entity(ctx.entity)
            .get::<Self>()
            .expect("no instance of MediaRelation on this entity");

        RelationCollectionTarget::decrement_or(world.reborrow(), media, studio, ctx.entity);
        RelationCollectionTarget::decrement_or(world.reborrow(), studio, media, ctx.entity);
    }
}
