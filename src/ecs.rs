use agb::hash_map::HashMap;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Entity {
  pub id: i32,
}

pub trait HasEntity {
  fn entity(&self) -> Entity;
}

pub trait EntityAccessor<T>: HasEntity + Sized {
  fn component(&self) -> &Map<T>;

  fn get(&self) -> Option<&T> {
    self.component().get(&self.entity())
  }
}

pub trait MutEntityAccessor<T>: HasEntity + Sized {
  fn component_mut(&mut self) -> &mut Map<T>;

  fn set(&mut self, val: T) -> &mut Self {
    let en = self.entity();
    let mut component = self.component_mut();
    component.insert(en, val);
    self
  }

  fn get_mut(&mut self) -> Option<&mut T> {
    let en = self.entity().clone();
    self.component_mut().get_mut(&en)
  }

  fn remove(&mut self) -> Option<T> {
    let en = self.entity().clone();
    self.component_mut().remove(&en)
  }
}

pub trait EntityDataBase<World> where World: WorldBase {
  fn new(world: &World, en: Entity) -> Self;
}

pub trait MutEntityDataBase<World> where World: WorldBase {
  fn new(world: &mut World, en: Entity) -> Self;
}

pub trait WorldBase: Sized {
  type Components;
  type EntityData: EntityDataBase<Self>;
  type MutEntityData: MutEntityDataBase<Self>;
  type Res;

  fn claim_next_entity_id(&mut self) -> i32;
  fn entities_mut(&mut self) -> &mut Entities;
  fn frame(&mut self, res: Self::Res);

  fn build_entity(&mut self) -> Self::MutEntityData {
    let en = Entity { id: self.claim_next_entity_id() };
    self.entities_mut().insert(en, ());
    Self::MutEntityData::new(self, en)
  }

  fn entity_data(&self, en: Entity) -> Self::EntityData {
    Self::EntityData::new(self, en)
  }

  fn entity_data_mut(&mut self, en: Entity) -> Self::MutEntityData {
    Self::MutEntityData::new(self, en)
  }
}

pub type Map<T> = HashMap<Entity, T>;
pub type Entities = Map<()>;