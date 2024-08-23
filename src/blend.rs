use agb::{
  display::blend::{Blend, Layer},
  fixnum::Num,
};

type OpacityNum = Num<u8, 4>;

mod opacity_num {
  use super::OpacityNum;

  pub const ZERO: OpacityNum = OpacityNum::from_raw(0);
  pub const ONE: OpacityNum = OpacityNum::from_raw(1 << 4);
  pub const MIN_INC: OpacityNum = OpacityNum::from_raw(1);
}

pub struct ManagedBlend<'o> {
  blend: Blend<'o>,
  top_opacity: OpacityNum,
}

impl<'o> ManagedBlend<'o> {
  pub fn new(blend: Blend<'o>) -> Self {
    let mut managed_blend = Self {
      blend,
      top_opacity: opacity_num::ONE,
    };
    managed_blend.update_blend_weight();
    managed_blend
  }

  pub fn get_top_opacity(&self) -> OpacityNum {
    self.top_opacity
  }

  pub fn set_top_opacity(&mut self, value: OpacityNum) {
    self.top_opacity = value.clamp(opacity_num::ZERO, opacity_num::ONE);
    self.update_blend_weight();
  }

  pub fn min_inc_top_opacity(&mut self) {
    self.set_top_opacity(self.top_opacity + opacity_num::MIN_INC);
  }

  pub fn min_dec_top_opacity(&mut self) {
    if self.top_opacity != opacity_num::ZERO {
      self.set_top_opacity(self.top_opacity - opacity_num::MIN_INC);
    }
  }

  pub fn commit(&mut self) {
    self.blend.commit();
  }

  fn update_blend_weight(&mut self) {
    self.blend.set_blend_weight(Layer::Top, self.top_opacity);
    self.blend.set_blend_weight(Layer::Bottom, opacity_num::ONE - self.top_opacity);
  }
}
