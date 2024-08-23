use alloc::boxed::Box;
use core::mem::transmute;
use agb::{display::{
  tiled::{MapLoan, VRamManager, RegularMap, TileSet, TileSetting},
  tile_data::TileData,
  palette16::Palette16,
}, fixnum::{Vector2D, Rect}};
use crate::{
  math::{PosNum, ZERO, MIN_INC},
  camera::Camera,
  collision::{CollideTileType, CollideTilemap},
};

#[derive(Clone, Copy)]
pub enum FlipTile<I> {
  N(I),
  X(I),
  Y(I),
  XY(I),
}

#[derive(Clone, Copy)]
pub struct Metatile {
  pub ul: FlipTile<usize>,
  pub ur: FlipTile<usize>,
  pub ll: FlipTile<usize>,
  pub lr: FlipTile<usize>,
}

pub struct TileSetData {
  pub metatiles: &'static [Metatile],
  pub palettes: &'static [Palette16],
  pub tile_data: &'static TileData,
}

#[derive(Clone, Copy)]
pub struct Tilemap {
  data: &'static [FlipTile<u8>],
  background_data: Option<&'static [FlipTile<u8>]>,
  foreground_data: Option<&'static [FlipTile<u8>]>,
  collision_data: &'static [CollideTileType],
  width: usize,
  height: usize,
  tileset: &'static TileSet<'static>,
  tileset_data: &'static TileSetData,
}


impl<I> FlipTile<I> {
  pub fn idx(self) -> I {
    match self {
      Self::N(idx) => idx,
      Self::X(idx) => idx,
      Self::Y(idx) => idx,
      Self::XY(idx) => idx,
    }
  }

  pub fn x_flipped(self) -> bool {
    match self {
      Self::X(_) => true,
      Self::XY(_) => true,
      _ => false,
    }
  }

  pub fn y_flipped(self) -> bool {
    match self {
      Self::Y(_) => true,
      Self::XY(_) => true,
      _ => false,
    }
  }

  pub fn flip_x(self) -> Self {
    match self {
      Self::N(idx) => Self::X(idx),
      Self::X(idx) => Self::N(idx),
      Self::Y(idx) => Self::XY(idx),
      Self::XY(idx) => Self::Y(idx),
    }
  }

  pub fn flip_y(self) -> Self {
    match self {
      Self::N(idx) => Self::Y(idx),
      Self::Y(idx) => Self::N(idx),
      Self::X(idx) => Self::XY(idx),
      Self::XY(idx) => Self::X(idx),
    }
  }
}

impl Metatile {
  pub const fn new(ul: FlipTile<usize>, ur: FlipTile<usize>, ll: FlipTile<usize>, lr: FlipTile<usize>) -> Self {
    Self{ul, ur, ll, lr}
  }

  pub fn flip_x(self) -> Self {
    Self {
      ul: self.ur.flip_x(),
      ur: self.ul.flip_x(),
      ll: self.lr.flip_x(),
      lr: self.ll.flip_x(),
    }
  }

  pub fn flip_y(self) -> Self {
    Self {
      ul: self.ll.flip_y(),
      ur: self.lr.flip_y(),
      ll: self.ul.flip_y(),
      lr: self.ur.flip_y(),
    }
  }
}

impl Tilemap {
  pub const fn new(
      data: &'static [FlipTile<u8>],
      bg: Option<&'static [FlipTile<u8>]>,
      fg: Option<&'static [FlipTile<u8>]>,
      col: &'static [CollideTileType],
      width: usize,
      tileset_data: &'static TileSetData) -> Self {
    Tilemap {
      data,
      background_data: bg,
      foreground_data: fg,
      collision_data: col,
      width,
      height: data.len() / width,
      tileset: &tileset_data.tile_data.tiles,
      tileset_data,
    }
  }

  pub fn primary_tile_fn<'a>(&'a self) -> Box<dyn Fn(Vector2D<i32>) -> (&'a TileSet<'a>, TileSetting) + 'a> {
    let self_clone = self.clone();
    Box::new(move |pos| {
      (
        self_clone.tileset,
        self_clone.get_tile(self_clone.data, pos),
      )
    })
  }

  pub fn background_tile_fn<'a>(&'a self) -> Box<dyn Fn(Vector2D<i32>) -> (&'a TileSet<'a>, TileSetting) + 'a> {
    let self_clone = self.clone();
    if let Some(data) = self.background_data {
      Box::new(move |pos| {
        (
          self_clone.tileset,
          self_clone.get_tile(data, pos),
        )
      })
    } else {
      Box::new(move |_| (self_clone.tileset, TileSetting::BLANK))
    }
  }

  pub fn foreground_tile_fn<'a>(&'a self) -> Box<dyn Fn(Vector2D<i32>) -> (&'a TileSet<'a>, TileSetting) + 'a> {
    let self_clone = self.clone();
    if let Some(data) = self.foreground_data {
      Box::new(move |pos| {
        (
          self_clone.tileset,
          self_clone.get_tile(data, pos),
        )
      })
    } else {
      Box::new(move |_| (self_clone.tileset, TileSetting::BLANK))
    }
  }

  fn get_tile(&self, data: &'static [FlipTile<u8>], pos: Vector2D<i32>) -> TileSetting {
    let metatile_pos = pos / 2;
    let lower = pos.y % 2 == 1;
    let right = pos.x % 2 == 1;
    if metatile_pos.x < 0 || metatile_pos.x >= self.width as i32 || metatile_pos.y < 0 || metatile_pos.y >= self.height as i32 {
      return TileSetting::BLANK;
    }
    let metatile_flip_idx = data[metatile_pos.x as usize + self.width * metatile_pos.y as usize];
    if metatile_flip_idx.idx() > 0 {
      let metatile = {
        let metatile_idx = (metatile_flip_idx.idx() - 1) as usize;
        let mut metatile = self.tileset_data.metatiles[metatile_idx];
        if metatile_flip_idx.x_flipped() {
          metatile = metatile.flip_x();
        }
        if metatile_flip_idx.y_flipped() {
          metatile = metatile.flip_y();
        }
        metatile
      };
      let tile_settings = self.tileset_data.tile_data.tile_settings;
      let tile_idx = match (lower, right) {
        (false, false) => metatile.ul,
        (false, true) => metatile.ur,
        (true, false) => metatile.ll,
        (true, true) => metatile.lr,
      };

      let result = Self::flipped_tile_settings(tile_settings, tile_idx);
      result
    } else {
      TileSetting::BLANK
    }
  }

  pub fn load_tileset_palette(&self, vram: &mut VRamManager) {
    vram.set_background_palettes(self.tileset_data.palettes);
  }

  pub fn set_camera_limits(&self, camera: &mut Camera) {
    camera.set_limits((PosNum::new(self.width as i32 * 16), PosNum::new(self.height as i32 * 16)).into())
  }

  fn flipped_tile_settings(tile_settings: &[TileSetting], tile_idx: FlipTile<usize>) -> TileSetting {
    if tile_idx.idx() > 0 {
      let mut tile_setting = tile_settings[tile_idx.idx() - 1];
      if tile_idx.x_flipped() {
        tile_setting = tile_setting.hflip(true);
      }
      if tile_idx.y_flipped() {
        tile_setting = tile_setting.vflip(true);
      }
      tile_setting
    } else {
      TileSetting::BLANK
    }
  }
}

impl Into<CollideTilemap> for Tilemap {
  fn into(self) -> CollideTilemap {
    CollideTilemap {
      data: self.collision_data,
      width: self.width,
      height: self.height,
    }
  }
}
