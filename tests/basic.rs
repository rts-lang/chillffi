use std::sync::Once;
use chillffi::ffi::value::{Type, Value};
use chillffi::{ffi, setupZygote};
// =================================================================================================

static Init: Once = Once::new();

/// Initializes the FFI environment once before running tests.
/// 
/// Uses Once for a safe one-time execution of setupZygote().
fn setup() -> ()
{
  Init.call_once(|| {
    setupZygote().expect("Failed to setup zygote");
  });
}

// =================================================================================================

/// Checks calling the sqrt function from the libm library.
#[test]
fn testSqrt() -> ()
{
  setup();

  let result: Value = ffi!{
    let libm: Library = Library::load("libm.so.6")?;
    let args: Vec<Value> = vec![Value::F64(4.0)];
    Ok(libm.call("sqrt", args, Type::F64)?)
  }.expect("FFI call failed");

  if let Value::F64(val) = result {
    assert!((val - 2.0).abs() < f64::EPSILON);
  } else {
    panic!("Expected F64");
  }
}

/// Checks calling the abs function from the libm library.
#[test]
fn testAbs() -> ()
{
  setup();
  
  let result: Value = ffi!{
    let libm: Library = Library::load("libm.so.6")?;
    let args: Vec<Value> = vec![Value::I32(-5)];
    Ok(libm.call("abs", args, Type::I32)?)
  }.expect("FFI call failed");

  if let Value::I32(val) = result {
    assert_eq!(val, 5);
  } else {
    panic!("Expected I32");
  }
}

// =================================================================================================

/// Checks repeated calls inside a single ffi!{} - uses cached dlopen.
#[test]
fn testMultipleCallsInSingleFFI() -> ()
{
  setup();

  let results: Vec<Value> = ffi!{
    let libm: Library = Library::load("libm.so.6")?;
    let mut outputs: Vec<Value> = Vec::with_capacity(10);

    // 10 consecutive libm.call() calls with a single loaded library
    for i in 1..=10 
    {
      let input: f64 = (i * i) as f64;
      let args: Vec<Value> = vec![Value::F64(input)];
      let res: Value = libm.call("sqrt", args, Type::F64)?;
      outputs.push(res);
    }

    Ok(outputs)
  }.expect("Batch FFI call failed");

  assert_eq!(results.len(), 10);

  for (i, val) in results.into_iter().enumerate() 
  {
    let expected: f64 = (i + 1) as f64;
    if let Value::F64(actual) = val {
      assert!((actual - expected).abs() < f64::EPSILON, "Expected {}, got {}", expected, actual);
    } else {
      panic!("Expected Value::F64 at index {}", i);
    }
  }
}

// =================================================================================================

/// Checks passing Value::None as an argument - should return an error.
#[test]
fn testNoneArgumentFails() -> ()
{
  setup();

  let result: Result<Value, _> = ffi!{
    let libm: Library = Library::load("libm.so.6")?;
    let args: Vec<Value> = vec![Value::None];
    let res: Value = libm.call("sqrt", args, Type::F64)?;
    Ok(res)
  };

  assert!(result.is_err(), "FFI call with Value::None should fail");
}

// =================================================================================================