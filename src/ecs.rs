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

pub type Map<T> = HashMap<Entity, T>;
pub type Entities = Map<()>;