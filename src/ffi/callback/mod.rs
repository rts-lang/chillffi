pub mod sendable;
pub mod addressing;
pub mod erased;
pub mod error;
// =================================================================================================
use crate::ffi::callback::error::CallError;
use crate::ffi::callback::erased::ErasedCallable;
use crate::ffi::callback::addressing::resolveRelative;
use crate::ffi::types::primitive::DynamicList;
use crate::ffi::types::Type;
use crate::ffi::types::Value;
use crate::ffi::types::primitive::{Primitive};
use serde::{Deserialize, Serialize};
// =================================================================================================

/// Re-exported so macro-generated code can reach these without requiring the
/// call site to have `serde`/`bincode` directly in scope.
#[doc(hidden)]
pub mod __reexport
{
  pub use bincode;
  pub use serde;
}

// =================================================================================================

/// Object-safe equivalent of `Fn(Args) -> Output`, callable through a trait object.
///
/// Inside this crate it has exactly one instantiation that matters:
/// `Callable<CallbackArgs, Value>` — the fully dynamic form the clone's
/// dispatcher holds. Macro-generated code never implements it directly: the
/// expansion runs in *foreign* crates where `Value` (`pub(crate)`) cannot
/// even be named; the bridge from the typed macro-generated entry point to
/// this dynamic form is `ErasedCallable` + `StateFnAdapter`.
pub trait Callable<Args, Output>: Send
{
  /// Executes the captured closure with the provided arguments.
  fn call(&self, args: Args) -> Output;
}

// =================================================================================================

/// Wire format: a relative pointer to the concrete decode function, both
/// sanity tags, and the captured state's own serialized bytes.
#[derive(Serialize, Deserialize)]
#[doc(hidden)]
pub(super) struct Envelope
{
  /// Offset of the target decode function relative to the module base.
  pub relativeOffset: usize,

  /// Hash of the argument and return types.
  pub argsOutputTag: u64,

  /// Hash of the source code location where the callback was defined.
  pub siteTag: u64,

  /// Serialized state of the captured variables.
  pub bytes: Vec<u8>
}

// =================================================================================================

/// Decodes bytes produced by [`Sendable::encode`] into a callable object.
/// Called inside the zygote clone after receiving the bytes over IPC —
/// requires no startup registration of any kind in that process.
///
/// Not generic over `Args`/`Output` any more: the fn pointer it transmutes
/// to is generated in a foreign crate and must have a *nameable* signature,
/// so the erased [`ErasedCallable`] is the return type.
pub fn decode(bytes: &[u8]) -> Result<ErasedCallable, CallError>
{
  let (envelope, _): (Envelope, usize) = bincode::serde::decode_from_slice(bytes, bincode::config::standard())
    .map_err(|e| CallError::Decode(e.to_string()))?;

  type DecodeFn = fn(u64, u64, &[u8]) -> Result<ErasedCallable, CallError>;

  let absoluteAddr: usize = resolveRelative(envelope.relativeOffset);
  // Safety: `absoluteAddr` was produced by `relativeOffsetOf` from a valid
  // `fn` item pointer in this exact executable, transmitted, and resolved back.
  //
  // Soundness relies strictly on both processes running the identical binary file
  // (zygote guarantees this via re-exec of `current_exe()`).
  //
  // As a second line of defense, the target function re-checks both the
  // call-site tag and the argument/return type tag before it deserializes
  // anything.
  let decodeFn: DecodeFn = unsafe { std::mem::transmute(absoluteAddr) };
  decodeFn(envelope.siteTag, envelope.argsOutputTag, &envelope.bytes)
}

// =================================================================================================

/// Wraps a closure so it can cross the zygote fork:
/// `callback!([x: i32] |args: T| -> U { ... })`.
///
/// Captures must be listed explicitly and their types must impl `Clone +
/// Serialize + DeserializeOwned` — `call` takes `&self`, not `self` (the same
/// closure can be invoked any number of times, e.g. once per comparison in a
/// `qsort`), so the captured tuple is cloned out on each call rather than moved.
///
/// The expansion never mentions `Value` (a `pub(crate)` type, unnameable in
/// foreign crates) and needs neither `serde` nor `bincode` at the call site:
/// captures travel as a plain tuple (serde has blanket impls for those), and
/// bincode is reached through `__reexport`.
/// 
/// todo desc - следует описать между строк не много, но что в целом происходит тут.
#[macro_export]
macro_rules! callback
{
  ($scope:expr, [$($name:ident : $ty:ty),* $(,)?] |$($argName:ident : $argTy:ty),* $(,)?| -> $retTy:ty $body:block) =>
  {
    {
      // Typed entry point: extracts the declared arguments from the dynamic
      // `CallbackArgs` by position and runs the original body. Only public
      // API is touched here (`CallbackArgs::get`, `Primitive::TypeTag`).
      #[allow(unused_variables, unused_mut)]
      fn __callTyped(
        state: &($($ty,)*),
        args: &$crate::ffi::types::primitive::DynamicList
      ) -> $retTy
      {
        // Clone the captured tuple out — the same closure may run many times.
        let ($($name,)*) = state.clone();

        let mut __i: usize = 0;
        $(
          let $argName: $argTy = args
            .get(__i)
            .expect(concat!("callback arg ", stringify!($argName), ": expected ", stringify!($argTy)));
          __i += 1;
        )*

        let result: $retTy = { $body };
        result
      }

      // Signature must exactly match `DecodeFn` in `decode`:
      // fn(u64 /*siteTag*/, u64 /*argsOutputTag*/, &[u8]) -> Result<ErasedCallable, CallError>.
      // No `Value` anywhere — that is what makes it compilable in foreign crates.
      fn __callDecode(
        siteTag: u64,
        argsOutputTag: u64,
        bytes: &[u8]
      ) -> ::std::result::Result<
        $crate::ffi::callback::erased::ErasedCallable,
        $crate::ffi::callback::error::CallError
      >
      {
        let expectedSiteTag: u64 = $crate::ffi::callback::addressing::tagOf(
          concat!(file!(), ":", line!(), ":", column!())
        );
        if siteTag != expectedSiteTag {
          return ::std::result::Result::Err(
            $crate::ffi::callback::error::CallError::TypeMismatch { tag: siteTag }
          );
        }

        let expectedTypesTag: u64 = $crate::ffi::callback::addressing::typesTagOf(
          &[ $( <$argTy as $crate::ffi::types::primitive::Primitive>::TypeTag ),* ],
          &<$retTy as $crate::ffi::types::primitive::Primitive>::TypeTag
        );
        if argsOutputTag != expectedTypesTag {
          return ::std::result::Result::Err(
            $crate::ffi::callback::error::CallError::ArgsOutputMismatch
          );
        }

        let (state, _): (($($ty,)*), usize) =
          $crate::ffi::callback::__reexport::bincode::serde::decode_from_slice(
            bytes,
            $crate::ffi::callback::__reexport::bincode::config::standard()
          ).map_err(|e| $crate::ffi::callback::error::CallError::Decode(
            ::std::string::ToString::to_string(&e)
          ))?;

        ::std::result::Result::Ok(
          $crate::ffi::callback::erased::ErasedCallable::fromStateAndFn(state, __callTyped)
        )
      }

      let siteTag: u64 = $crate::ffi::callback::addressing::tagOf(
        concat!(file!(), ":", line!(), ":", column!())
      );
      let relativeOffset: usize = $crate::ffi::callback::addressing::relativeOffsetOf(
        __callDecode as *const () as usize
      );
      let argTypes: ::std::vec::Vec<$crate::ffi::types::Type> =
        vec![ $( <$argTy as $crate::ffi::types::primitive::Primitive>::TypeTag ),* ];
      let returnType: $crate::ffi::types::Type =
        <$retTy as $crate::ffi::types::primitive::Primitive>::TypeTag;

      $scope.callback(
        $crate::ffi::callback::sendable::Sendable::new(
          relativeOffset,
          siteTag,
          argTypes,
          returnType,
          ($($name,)*),
          __callTyped
        )
      )
      //
    }
  };
  //
}

// =================================================================================================