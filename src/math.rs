use core::convert::From;
use agb::fixnum::{
  Num,
  FixedWidthUnsignedInteger,
};

pub type PosNum = Num<i32, 8>;

pub const ZERO: PosNum = PosNum::from_raw(0);
pub const MIN_INC: PosNum = PosNum::from_raw(1);

pub const fn const_num_u32<const Bits: usize>(ipart: u32, fpart: u32) -> Num<u32, Bits> {
  let places = {
    let mut places = 0;
    let mut fpart = fpart;
    while fpart != 0 {
      fpart = fpart / 10;
      places += 1;
    }
    places
  };

  let (fshift, shifted_fpart) = {
    let mut fshift = 0;
    let mut shifted_fpart = fpart;
    let dec_mod = 10_u32.pow(places);
    while (shifted_fpart % dec_mod) != 0 && fshift < Bits {
      shifted_fpart = shifted_fpart << 1;
      fshift += 1;
    }
    (fshift, shifted_fpart / dec_mod)
  };
  let shifted_ipart = ipart << fshift;

  Num::<u32, Bits>::from_raw((shifted_ipart + shifted_fpart) << (Bits - fshift))
}

pub const fn const_num_i32<const Bits: usize>(ipart: i32, fpart: i32) -> Num<i32, Bits> {
  let places = {
    let mut places = 0;
    let mut fpart = fpart;
    while fpart != 0 {
      fpart = fpart / 10;
      places += 1;
    }
    places
  };

  let (fshift, shifted_fpart) = {
    let mut fshift = 0;
    let mut shifted_fpart = fpart;
    let dec_mod = 10_i32.pow(places);
    while (shifted_fpart % dec_mod) != 0 && fshift < Bits {
      shifted_fpart = shifted_fpart << 1;
      fshift += 1;
    }
    (fshift, shifted_fpart / dec_mod)
  };
  let shifted_ipart = ipart << fshift;

  Num::<i32, Bits>::from_raw((shifted_ipart + shifted_fpart) << (Bits - fshift))
}
