use std::{fmt::{Display, format, Formatter}, fs::File, io::{BufWriter, Result, Write}, io, iter::Map};
use std::collections::HashMap;
use std::io::{Error, ErrorKind};

pub fn export_world(components: &[&'static str], resources: &[&'static str], systems: Vec<(&'static str, &[&'static str])>, imports: &[&'static str], out_dir: &str) -> Result<()> {
  let component_id_map = type_id_map(components);
  let resource_id_map = type_id_map(resources);
  let systems = {
    let mut new_systems = vec![];
    for (name, types) in systems {
      let types: Vec<SystemParam> = types.iter().map(|t| SystemParam::parse(*t, &component_id_map, &resource_id_map)).collect();
      new_systems.push(System {
        name,
        params: types,
      });
    }
    new_systems
  };

  let output_file = File::create(format!("{out_dir}/world.rs"))?;
  let mut writer = BufWriter::new(output_file);

  for import in imports {
    writeln!(&mut writer, "use {};", import)?;
  }

  writeln!(&mut writer, r#"
use agb_ext::ecs::{{
  Entity, HasEntity, EntityAccessor, MutEntityAccessor, EntityDataBase, MutEntityDataBase,
  WorldBase, Map, Entities
}};

pub struct EntityData<'w, 'o> {{
  world: &'w World<'o>,
  en: Entity
}}

pub struct MutEntityData<'w, 'o> {{
  world: &'w mut World<'o>,
  en: Entity
}}

impl<'w, 'o> EntityDataBase<World<'o>> for EntityData<'w, 'o> {{
  fn new(world: &'w World<'o>, en: Entity) -> Self {{ Self{{ world, en }} }}
}}

impl<'w, 'o> MutEntityDataBase<World<'o>> for MutEntityData<'w, 'o> {{
  fn new(world: &'w mut World<'o>, en: Entity) -> Self {{ Self{{ world, en }} }}
}}

impl<'w, 'o> HasEntity for EntityData<'w, 'o> {{
  fn entity(&self) -> Entity {{ self.en }}
}}

impl<'w, 'o> HasEntity for MutEntityData<'w, 'o> {{
  fn entity(&self) -> Entity {{ self.en }}
}}"#)?;

  for (component, id) in component_id_map.iter() {
    let (component, id) = (*component, id.clone());
    writeln!(&mut writer, r#"
impl <'w, 'o> EntityAccessor<{component}> for EntityData<'w, 'o> {{
  fn component(&self) -> &Map<{component}> {{ &self.world.components.{id} }}
}}

impl <'w, 'o> EntityAccessor<{component}> for MutEntityData<'w, 'o> {{
  fn component(&self) -> &Map<{component}> {{ &self.world.components.{id} }}
}}

impl <'w, 'o> MutEntityAccessor<{component}> for MutEntityData<'w, 'o> {{
  fn component(&mut self) -> &mut Map<{component}> {{ &mut self.world.components.{id} }}
}}"#)?;
  }

  write!(&mut writer, "type Components<'o> = (")?;
  for component in components {
    write!(&mut writer, "{component}, ")?;
  }
  writeln!(&mut writer, ");")?;

  write!(&mut writer, "type Res<'o> = (")?;
  for resource in resources {
    write!(&mut writer, "{resource}, ")?;
  }
  writeln!(&mut writer, ");")?;

  writeln!(&mut writer, r#"
pub struct World<'o> {{
  pub(self) components: Components<'o>,
  entities: Entities,
  next_entity_id: i32,
}}

impl<'o> WorldBase for World<'o> {{
  type Components = Components<'o>;
  type Res = Res<'o>;
  type EntityData<'w> = EntityData<'w, 'o>;
  type MutEntityData<'w> = MutEntityData<'w, 'o>;

  fn claim_next_entity_id(&mut self) -> i32 {{
    let entity_id = next_entity_id.clone();
    next_entity_id += 1;
    entity_id
  }}

  fn entities_mut(&mut self) -> &mut Entities {{ &mut self.entities }}

  fn frame(&mut self, res: Res) {{"#)?;

  let mut type_stack: Vec<SystemParam> = vec![];
  let first_var_name = 'a' as u32;
  for system in systems {
    let first_discrepancy = {
      let mut first_discrepancy = 0;
      for param_type in type_stack.iter() {
        if Some(param_type) == system.params.get(first_discrepancy.clone()) {
          first_discrepancy += 1;
        } else {
          break;
        }
      }
      first_discrepancy
    };
    for i in (first_discrepancy..type_stack.len()).rev() {
      writeln!(&mut writer, "{}}}", tab_pad(2 + i))?;
      type_stack.pop();
    }

    for (i, param_type) in system.params[first_discrepancy..].iter().enumerate() {
      type_stack.push(param_type.clone());
      if first_discrepancy == 0 && i == 0 {
        match param_type {
          SystemParam::Component(t) if t.param_type == ParamType::Ref => writeln!(&mut writer, "    for (en, a) in self.components.{}.iter() {{", t.idx),
          SystemParam::Component(t) if t.param_type == ParamType::MutRef => writeln!(&mut writer, "    for (en, a) in self.components.{}.iter_mut() {{", t.idx),
          _ => param_type.write_var(&mut writer, 'a', tab_pad(2)),
        }?;
      } else {
        let offset = i + first_discrepancy;
        param_type.write_var(&mut writer, char::from_u32(first_var_name + offset as u32).expect("Cannot create char correctly"), tab_pad(2 + offset))?;
      }
    }

    write!(&mut writer, "{}{}(", tab_pad(2 + type_stack.len()), system.name)?;
    for i in 0..type_stack.len() {
      write!(&mut writer, "{}, ", char::from_u32(first_var_name + i as u32).expect("Cannot create char correctly"))?;
    }
    writeln!(&mut writer, ");")?;
  }
  for i in (0..type_stack.len() + 2).rev() {
    writeln!(&mut writer, "{}}}", tab_pad(i))?;
  }

  Ok(())
}

fn tab_pad(amt: usize) -> String {
  "  ".repeat(amt)
}

fn type_id_map(types: &[&'static str]) -> HashMap<&'static str, usize> {
  let mut type_id_map = HashMap::new();
  for (idx, type_name) in types.iter().enumerate() {
    type_id_map.insert(*type_name, idx);
  }
  type_id_map
}

struct System {
  name: &'static str,
  params: Vec<SystemParam>,
}

#[derive(PartialEq, Clone)]
enum SystemParam {
  Component(IndexedType),
  Resource(IndexedType),
  Entities,
  World(ParamType),
}

#[derive(PartialEq, Clone)]
struct IndexedType {
  idx: usize,
  name: String,
  param_type: ParamType,
}

#[derive(PartialEq, Clone, Debug)]
enum ParamType {
  Ref,
  MutRef,
  Option,
  MutOption,
  Map,
  MutMap,
}

impl SystemParam {
  pub fn parse(src: &'static str, component_id_map: &HashMap<&'static str, usize>, resource_id_map: &HashMap<&'static str, usize>) -> Self {
    let (param_type, type_name) = {
      if src.starts_with("&mut Map<") {
        (ParamType::MutMap, &src[9..(src.len() - 1)])
      } else if src.starts_with("& Map<") {
        (ParamType::MutMap, &src[6..(src.len() - 1)])
      } else if src.starts_with("Option<") {
        let inner = &src[7..(src.len() - 1)];
        if inner.starts_with("&mut ") {
          (ParamType::MutOption, &inner[5..])
        } else if inner.starts_with("&") {
          (ParamType::Option, &inner[1..])
        } else {
          panic!("Cannot parse Option type {}", inner)
        }
      } else if src.starts_with("&mut ") {
        (ParamType::MutRef, &src[5..])
      } else if src.starts_with("&") {
        (ParamType::Ref, &src[1..])
      } else {
        panic!("Cannot parse type {}", src)
      }
    };

    if type_name == "World" {
      Self::World(param_type)
    } else if type_name == "Entities" {
      Self::Entities
    } else if let Some(component_id) = component_id_map.get(type_name) {
      Self::Component(IndexedType {
        idx: component_id.clone(),
        name: type_name.to_string(),
        param_type
      })
    } else if let Some(resource_id) = resource_id_map.get(type_name) {
      Self::Resource(IndexedType {
        idx: resource_id.clone(),
        name: type_name.to_string(),
        param_type
      })
    } else {
      panic!("Type {} is not known to this World", type_name)
    }
  }

  pub fn write_var(&self, writer: &mut BufWriter<File>, var_name: char, tab_pad: String) -> Result<()> {
    match self {
      Self::Component(t) => match t.param_type {
        ParamType::Ref => writeln!(writer, "{tab_pad}if let Some({var_name}) = self.components.{}.get(en) {{", t.idx),
        ParamType::MutRef => writeln!(writer, "{tab_pad}if let Some({var_name}) = self.components.{}.get_mut(en) {{", t.idx),
        ParamType::Option => writeln!(writer, "{tab_pad}let {var_name} = self.components.{}.get(en);\n{tab_pad}{{", t.idx),
        ParamType::MutOption => writeln!(writer, "{tab_pad}let {var_name} = self.components.{}.get_mut(en);\n{tab_pad}{{", t.idx),
        ParamType::Map => writeln!(writer, "{tab_pad}let {var_name} = &self.components.{};\n{tab_pad}{{", t.idx),
        ParamType::MutMap => writeln!(writer, "{tab_pad}let {var_name} = &mut self.components.{};\n{tab_pad}{{", t.idx),
      },
      Self::Resource(t) => match &t.param_type {
        ParamType::Ref => writeln!(writer, "{tab_pad}let {var_name} = res.{};\n{tab_pad}{{", t.idx),
        ParamType::MutRef => writeln!(writer, "{tab_pad}let {var_name} = res.{};\n{tab_pad}{{", t.idx),

        invalid => Err(Error::new(ErrorKind::InvalidInput, format!("Cannot borrow resource as {:?}", invalid))),
      },
      Self::Entities => writeln!(writer, "{tab_pad}let {var_name} = &self.entities;\n{tab_pad}{{"),
      Self::World(p) => match &p {
        ParamType::Ref => writeln!(writer, "{tab_pad}let {var_name} = &self;\n{tab_pad}{{"),
        ParamType::MutRef => writeln!(writer, "{tab_pad}let {var_name} = &mut self;\n{tab_pad}{{"),

        invalid => Err(Error::new(ErrorKind::InvalidInput, format!("Cannot borrow world as {:?}", invalid))),
      }
    }
  }
}