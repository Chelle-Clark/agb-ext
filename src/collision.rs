use core::convert::Into;
use agb::{fixnum::{Vector2D, Num, Rect}, include_wav, input::ButtonController};
use crate::collision::CollisionLayer::Pipe;
use crate::math::{PosNum, const_num_i32, ZERO, MIN_INC};
use crate::ecs::Entity as EcsEntity;

#[derive(Clone, Copy, PartialEq)]
pub enum CollideTileType {
  Pass,
  Solid,
  LWall,
  RWall,
  Pipe,
  RSteepSlope,
  RLowSlope1,
  RLowSlope2,
  PipeSolid,
  LSteepSlope,
  LLowSlope1,
  LLowSlope2,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CollisionLayer {
  Normal,
  Pipe,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Pos(pub Vector2D<PosNum>);

#[derive(Clone, Copy, PartialEq)]
pub struct Vel(pub Vector2D<PosNum>);

#[derive(Clone, Copy, PartialEq)]
pub struct Acc(pub Vector2D<PosNum>);

#[derive(Clone, Copy, PartialEq)]
pub struct Size(pub Vector2D<PosNum>);

#[derive(Clone, Copy, PartialEq)]
pub struct OnGround(pub bool);

#[derive(Clone, Debug)]
pub struct Collision {
  pub x_seam: Option<i32>,
  pub y_seam: Option<i32>,
  pub snap_to_ground: bool,
}

pub struct CollideTilemap {
  pub data: &'static [CollideTileType],
  pub width: usize,
  pub height: usize,
}


pub trait Entity {
  fn move_by(&mut self, offset: Vector2D<PosNum>, snap_to_ground: bool);
  fn set_position(&mut self, position: Vector2D<PosNum>);
  fn position(&self) -> Vector2D<PosNum>;
  fn col_rect(&self) -> Rect<PosNum>;
  fn col_layer(&self) -> CollisionLayer { CollisionLayer::Normal }
}

pub trait ControllableEntity: Entity {
  fn propose_movement(&mut self, input: Option<&ButtonController>) -> Vector2D<PosNum>;

  fn physics_process(&mut self, tilemap: &CollideTilemap, input: Option<&ButtonController>) {
    let movement = self.propose_movement(input);
    let hitbox = self.col_rect();
    let col = tilemap.get_collision_seams(movement, hitbox, self.col_layer());
    self.move_by(move_and_collide(movement, hitbox, &col), col.snap_to_ground);
  }
}


pub mod system {
  use super::*;

  pub fn apply_vel(pos: &mut Pos, vel: &Vel) {
    pos.0 = pos.0 + vel.0;
  }

  pub fn apply_acc(vel: &mut Vel, acc: &Acc) {
    vel.0 = vel.0 + acc.0;
  }

  pub fn print_pos(en: &EcsEntity, pos: &Pos) {
    agb::println!("{:?}: {:?}", en, pos.0);
  }

  pub fn physics_process(pos: &Pos, vel: &mut Vel, size: &Size, col_layer: &CollisionLayer, on_ground: Option<&mut OnGround>, tilemap: &CollideTilemap) {
    let hitbox = Rect::new(pos.0, size.0);
    let col = tilemap.get_collision_seams(vel.0, hitbox, *col_layer);
    let new_vel = move_and_collide(vel.0, hitbox, &col);

    if vel.0.y > new_vel.y {
      if let Some(on_ground) = on_ground {
        on_ground.0 = true;
      }
    }

    vel.0 = new_vel;
  }
}


fn move_and_collide(movement: Vector2D<PosNum>, hitbox: Rect<PosNum>, col: &Collision) -> Vector2D<PosNum> {
  let mut actual = movement.clone();
  if let Some(x_collision) = col.x_seam {
    let desired_x = {
      let mut value = PosNum::new(x_collision);
      if movement.x > PosNum::new(0) {
        value -= hitbox.size.x;
      }
      value
    };
    actual.x = desired_x - hitbox.position.x;
  }
  if let Some(y_collision) = col.y_seam {
    let desired_y = {
      let mut value = PosNum::new(y_collision);
      if movement.y > PosNum::new(0) {
        value -= hitbox.size.y;
      }
      value
    };
    actual.y = desired_y - hitbox.position.y;
  }

  actual
}


impl CollideTilemap {
  const PX_PER_TILE: PosNum = const_num_i32(16, 0);

  fn get_collision_seams(&self, movement: Vector2D<PosNum>, hitbox: Rect<PosNum>, layer: CollisionLayer) -> Collision {
    let moving_left = movement.x < ZERO;
    let moving_up = movement.y < ZERO;
    let entered_x = {
      let cur_edge = {
        if !moving_left {
          hitbox.position.x + hitbox.size.x - MIN_INC
        } else {
          hitbox.position.x
        }
      };
      if (cur_edge / Self::PX_PER_TILE).floor() != ((cur_edge + movement.x) / Self::PX_PER_TILE).floor() {
        Some(((cur_edge + movement.x) / Self::PX_PER_TILE).floor())
      } else {
        None
      }
    };
    let entered_y = {
      let cur_edge = {
        if !moving_up {
          hitbox.position.y + hitbox.size.y - MIN_INC
        } else {
          hitbox.position.y
        }
      };
      if (cur_edge / Self::PX_PER_TILE).floor() != ((cur_edge + movement.y) / Self::PX_PER_TILE).floor() {
        Some(((cur_edge + movement.y) / Self::PX_PER_TILE).floor())
      } else {
        None
      }
    };

    let adjusted_hitbox = Rect::new(hitbox.position + movement, hitbox.size);
    let (tile_left_x, tile_right_x) = {
      let left_x = adjusted_hitbox.position.x;
      let right_x = left_x + hitbox.size.x - MIN_INC;
      ((left_x / Self::PX_PER_TILE).floor(), (right_x / Self::PX_PER_TILE).floor())
    };
    let (tile_up_y, tile_down_y) = {
      let up_y = adjusted_hitbox.position.y;
      let down_y = up_y + hitbox.size.y - MIN_INC;
      ((up_y / Self::PX_PER_TILE).floor(), (down_y / Self::PX_PER_TILE).floor())
    };
    let mut corner_y_seam = None;

    let mut x_seam = None;
    let mut y_seam = None;
    let mut snap_to_ground = false;
    for xi in tile_left_x..=tile_right_x {
      for yi in tile_up_y..=tile_down_y {
        if xi > 0 && xi < self.width as i32 && yi > 0 && yi < self.height as i32 {
          let tile_idx = xi as usize + yi as usize * self.width;
          let tile: CollideTileType = self.data[tile_idx];
          if tile.is_tile_colliding((xi, yi).into(), adjusted_hitbox, layer) {
            match (entered_x == Some(xi), entered_y == Some(yi)) {
              (false, false) => {
                if tile.is_nonstandard_hitbox() {
                  let specialized_col = tile.specialized_collide((xi, yi).into(), adjusted_hitbox, moving_left, moving_up);
                  snap_to_ground = specialized_col.snap_to_ground;
                  (x_seam, y_seam) = Self::handle_specialized_collide(specialized_col, x_seam, y_seam, moving_left, moving_up);
                }
              }
              (true, false) => {
                if tile.is_slope() {
                  let specialized_col = tile.specialized_collide((xi, yi).into(), adjusted_hitbox, moving_left, moving_up);
                  snap_to_ground = specialized_col.snap_to_ground;
                  (x_seam, y_seam) = Self::handle_specialized_collide(specialized_col, x_seam, y_seam, moving_left, moving_up);
                } else {
                  let new_seam = if moving_left { (xi + 1) * 16 } else { xi * 16 };
                  x_seam = Some(Self::stricter_seam(x_seam, new_seam, moving_left));
                }
              }
              (false, true) => {
                if tile.is_slope() {
                  let specialized_col = tile.specialized_collide((xi, yi).into(), adjusted_hitbox, moving_left, moving_up);
                  snap_to_ground = specialized_col.snap_to_ground;
                  (x_seam, y_seam) = Self::handle_specialized_collide(specialized_col, x_seam, y_seam, moving_left, moving_up);
                } else {
                  let new_seam = if moving_up { (yi + 1) * 16 } else { yi * 16 };
                  y_seam = Some(Self::stricter_seam(y_seam, new_seam, moving_up));
                }
              }
              (true, true) => {
                corner_y_seam = Some(if moving_up { (yi + 1) * 16 } else { yi * 16 });
              }
            }
          }
        }
      }
    }

    if corner_y_seam.is_some() && x_seam.is_none() && y_seam.is_none() {
      y_seam = corner_y_seam;
    }

    Collision { x_seam, y_seam, snap_to_ground }
  }

  fn stricter_seam(cur: Option<i32>, new: i32, moving_negative: bool) -> i32 {
    if let Some(cur) = cur {
      if moving_negative {
        cur.max(new)
      } else {
        cur.min(new)
      }
    } else {
      new
    }
  }

  fn handle_specialized_collide(specialized_col: Collision, x_seam: Option<i32>, y_seam: Option<i32>, moving_left: bool, moving_up: bool) -> (Option<i32>, Option<i32>) {
    let x_seam = if let Some(new_x_seam) = specialized_col.x_seam {
      Some(Self::stricter_seam(x_seam, new_x_seam, moving_left))
    } else {
      x_seam
    };
    let y_seam = if let Some(new_y_seam) = specialized_col.y_seam {
      Some(Self::stricter_seam(y_seam, new_y_seam, moving_up))
    } else {
      y_seam
    };

    (x_seam, y_seam)
  }
}

impl CollideTileType {
  pub fn is_nonstandard_hitbox(self) -> bool {
    match self {
      Self::LWall => true,
      Self::RWall => true,
      x if x.is_slope() => true,
      _ => false,
    }
  }

  pub fn is_slope(self) -> bool {
    [
      Self::LSteepSlope, Self::LLowSlope1, Self::LLowSlope2,
      Self::RSteepSlope, Self::RLowSlope1, Self::RLowSlope2
    ].contains(&self)
  }

  pub fn is_tile_colliding(self, pos: Vector2D<i32>, adjusted_hitbox: Rect<PosNum>, layer: CollisionLayer) -> bool {
    match self {
      Self::Pass => layer == CollisionLayer::Pipe,
      Self::Solid => true,
      Self::Pipe => false,
      Self::PipeSolid => layer == CollisionLayer::Normal,

      Self::LWall => {
        let pos = Vector2D::new(PosNum::new(pos.x * 16), PosNum::new(pos.y * 16));
        let self_rect = Rect::new(pos, (2, 16).into());
        adjusted_hitbox.touches(self_rect)
      }
      Self::RWall => {
        let pos = Vector2D::new(PosNum::new(pos.x * 16 + 14), PosNum::new(pos.y * 16));
        let self_rect = Rect::new(pos, (2, 16).into());
        adjusted_hitbox.touches(self_rect)
      }

      Self::RSteepSlope => {
        let relative_hitbox = Rect::new(adjusted_hitbox.position - (pos * 16).into(), adjusted_hitbox.size);
        let corner = Vector2D::new(relative_hitbox.position.x + relative_hitbox.size.x, relative_hitbox.position.y + relative_hitbox.size.y);
        corner.y > const_num_i32(16, 0) - corner.x
      }
      Self::LSteepSlope => {
        let relative_hitbox = Rect::new(adjusted_hitbox.position - (pos * 16).into(), adjusted_hitbox.size);
        let corner = Vector2D::new(relative_hitbox.position.x, relative_hitbox.position.y + relative_hitbox.size.y);
        corner.y > corner.x
      }

      Self::RLowSlope1 => {
        let relative_hitbox = Rect::new(adjusted_hitbox.position - (pos * 16).into(), adjusted_hitbox.size);
        let corner = Vector2D::new(relative_hitbox.position.x + relative_hitbox.size.x, relative_hitbox.position.y + relative_hitbox.size.y);
        corner.y > const_num_i32(16, 0) - (corner.x / const_num_i32(2, 0))
      }
      Self::RLowSlope2 => {
        let relative_hitbox = Rect::new(adjusted_hitbox.position - (pos * 16).into(), adjusted_hitbox.size);
        let corner = Vector2D::new(relative_hitbox.position.x + relative_hitbox.size.x, relative_hitbox.position.y + relative_hitbox.size.y);
        corner.y > const_num_i32(8, 0) - (corner.x / const_num_i32(2, 0))
      }

      Self::LLowSlope1 => true,
      Self::LLowSlope2 => true,
      Self::RLowSlope1 => true,
      Self::RLowSlope2 => true,
    }
  }

  fn specialized_collide(self, pos: Vector2D<i32>, adjusted_hitbox: Rect<PosNum>, moving_left: bool, _moving_up: bool) -> Collision {
    match self {
      Self::LWall => Collision {
        x_seam: Some(if moving_left { pos.x * 16 + 2 } else { pos.x * 16 }),
        y_seam: None,
        snap_to_ground: false,
      },
      Self::RWall => Collision {
        x_seam: Some(if moving_left { (pos.x + 1) * 16 } else { (pos.x + 1) * 16 - 2 }),
        y_seam: None,
        snap_to_ground: false,
      },

      Self::RSteepSlope => {
        let relative_hitbox = Rect::new(adjusted_hitbox.position - (pos * 16).into(), adjusted_hitbox.size);
        Collision {
          x_seam: None,
          y_seam: Some(16 * pos.y + (16 - (relative_hitbox.position.x + relative_hitbox.size.x).trunc())),
          snap_to_ground: true,
        }
      }
      Self::LSteepSlope => {
        let relative_hitbox = Rect::new(adjusted_hitbox.position - (pos * 16).into(), adjusted_hitbox.size);
        Collision {
          x_seam: None,
          y_seam: Some(16 * pos.y + relative_hitbox.position.x.trunc()),
          snap_to_ground: true,
        }
      }

      Self::RLowSlope1 => {
        let relative_hitbox = Rect::new(adjusted_hitbox.position - (pos * 16).into(), adjusted_hitbox.size);
        Collision {
          x_seam: None,
          y_seam: Some(16 * pos.y + (16 - (relative_hitbox.position.x + relative_hitbox.size.x).trunc() / 2)),
          snap_to_ground: true,
        }
      }
      Self::RLowSlope2 => {
        let relative_hitbox = Rect::new(adjusted_hitbox.position - (pos * 16).into(), adjusted_hitbox.size);
        Collision {
          x_seam: None,
          y_seam: Some(16 * pos.y + (8 - (relative_hitbox.position.x + relative_hitbox.size.x).trunc() / 2)),
          snap_to_ground: true,
        }
      }

      _ => Collision { x_seam: None, y_seam: None, snap_to_ground: false }
    }
  }
}
