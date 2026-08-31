use chillffi::ffi;
// =================================================================================================

/// Execute math functions from libm
fn main() -> ()
{
  // Call sqrt(4.0)
  let result: f64 = ffi!{
    let libm: Library = Library::load("libm.so.6")?;
    Ok( libm.call("sqrt").arg(4.0 as f64).result()? )
  }.expect("FFI call failed");

  println!("sqrt(4.0) = {}", result);
  assert!((result - 2.0).abs() < f64::EPSILON, "sqrt(4.0) != 2.0");

  // Call abs(-5)
  let result: i32 = ffi!{
    let libm: Library = Library::load("libm.so.6")?;
    Ok( libm.call("abs").arg(-5 as i32).result()? )
  }.expect("FFI call failed");

  println!("abs(-5) = {}", result);
  assert_eq!(result, 5, "abs(-5) != 5");

  //
  println!("All tests passed!");
}

// =================================================================================================