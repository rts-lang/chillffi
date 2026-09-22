#[path = "../platform/mod.rs"]
mod platform;
use crate::platform::platformExt;
// =================================================================================================
use chillffi::ffi;
use chillffi::ffi::types::primitive::{Arg, StructValue};
use chillffi::ffi::types::Type;
// =================================================================================================

/// todo desc
fn main() -> () {
  pointSum();
  pointTranslate();
  bigStruct();
  nestedStruct();
}

// =================================================================================================

/// todo desc
fn pointSum() -> () {
  let sum: i32 = ffi!(|scope| {
    scope.addSearchPath("examples/byValueStruct");
    let lib: Library = scope.load(platformExt!("libbyvalue"))?;
    let point: StructValue = StructValue::new([Arg::from(10i32), Arg::from(32i32)]);
    lib.call("point_sum").arg(point).result()
  }).expect("point_sum failed");
  assert_eq!(sum, 42);
  println!("ok: point_sum(by-value) -> {sum}");
}

/// todo desc
fn pointTranslate() -> () {
  let (x, y): (i32, i32) = ffi!(|scope| {
    scope.addSearchPath("examples/byValueStruct");
    let lib: Library = scope.load(platformExt!("libbyvalue"))?;
    let point: StructValue = StructValue::new([Arg::from(1i32), Arg::from(2i32)]);
    let out: StructValue = lib.call("point_translate").arg(point).arg(10i32).arg(20i32)
      .resultStruct(&[Type::I32, Type::I32])?;
    Ok((out.get(0)?, out.get(1)?))
  }).expect("point_translate failed");
  assert_eq!((x, y), (11, 22));
  println!("ok: point_translate(by-value) -> ({x}, {y})");
}

/// todo desc
fn bigStruct() -> () {
  let sum: f64 = ffi!(|scope| {
    scope.addSearchPath("examples/byValueStruct");
    let lib: Library = scope.load(platformExt!("libbyvalue"))?;
    let big: StructValue = StructValue::new([
      Arg::from(1.5f64), Arg::from(2.5f64), Arg::from(3.0f64), Arg::from(7i32),
    ]);
    lib.call("big_sum").arg(big).result()
  }).expect("big_sum failed");
  assert!((sum - 14.0).abs() < 1e-9);
  println!("ok: big_sum(by-value >16B) -> {sum}");

  let (a, tag): (f64, i32) = ffi!(|scope| {
    scope.addSearchPath("examples/byValueStruct");
    let lib: Library = scope.load(platformExt!("libbyvalue"))?;
    let big: StructValue = StructValue::new([
      Arg::from(1.0f64), Arg::from(2.0f64), Arg::from(3.0f64), Arg::from(4i32),
    ]);
    let out: StructValue = lib.call("big_scale").arg(big).arg(2.0f64)
      .resultStruct(&[Type::F64, Type::F64, Type::F64, Type::I32])?;
    Ok((out.get(0)?, out.get(3)?))
  }).expect("big_scale failed");
  assert!((a - 2.0).abs() < 1e-9);
  assert_eq!(tag, 4);
  println!("ok: big_scale(by-value return) -> a={a}, tag={tag}");
}

/// todo desc
fn nestedStruct() -> () {
  let scale: f32 = ffi!(|scope| {
    scope.addSearchPath("examples/byValueStruct");
    let lib: Library = scope.load(platformExt!("libbyvalue"))?;
    let nested: StructValue = StructValue::new([
      Arg::from(StructValue::new([Arg::from(3i32), Arg::from(4i32)])),
      Arg::from(2.5f32),
    ]);
    lib.call("nested_scale").arg(nested).result()
  }).expect("nested_scale failed");
  assert!((scale - 17.5).abs() < 1e-5);
  println!("ok: nested_scale(by-value nested) -> {scale}");

  let (ox, oy, sc): (i32, i32, f32) = ffi!(|scope| {
    scope.addSearchPath("examples/byValueStruct");
    let lib: Library = scope.load(platformExt!("libbyvalue"))?;
    let nested: StructValue = StructValue::new([
      Arg::from(StructValue::new([Arg::from(5i32), Arg::from(6i32)])),
      Arg::from(1.5f32),
    ]);
    let out: StructValue = lib.call("nested_double").arg(nested)
      .resultStruct(&[Type::structure([Type::I32, Type::I32]), Type::F32])?;
    let origin: StructValue = out.getStruct(0)?;
    Ok((origin.get(0)?, origin.get(1)?, out.get(1)?))
  }).expect("nested_double failed");
  assert_eq!((ox, oy), (10, 12));
  assert!((sc - 3.0).abs() < 1e-5);
  println!("ok: nested_double(by-value nested return) -> origin=({ox},{oy}) scale={sc}");
}

// =================================================================================================
