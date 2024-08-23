use agb::{
  fixnum::{Num, num},
  sound::mixer::{Mixer, SoundChannel},
};
use crate::math::const_num_u32;

pub struct Music {
  data: &'static [u8],
  loop_point: Num<u32, 8>,
}

const FREQ: u32 = 32768;
const FREQ_NUM: Num<u32, 8> = const_num_u32(FREQ,0);

impl Music {
  pub const fn new(data: &'static [u8], loop_point: Num<u32, 8>) -> Self {
    Self{data, loop_point}
  }

  pub fn play(&self, mixer: &mut Mixer) {
    let mut bgm = SoundChannel::new(self.data);
    bgm.stereo().should_loop().restart_point(self.loop_point * FREQ_NUM);
    let _ = mixer.play_sound(bgm);
  }

  pub fn play_high_priority(&self, mixer: &mut Mixer) {
    let mut bgm = SoundChannel::new_high_priority(self.data);
    bgm.stereo().should_loop().restart_point(self.loop_point);
    let _ = mixer.play_sound(bgm);
  }
}
