use crate::ffi::types::primitive::DynamicList;
use crate::ffi::types::Type;
use crate::ffi::types::Value;
use crate::ffi::types::primitive::{Primitive};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use fxhash::FxHasher;
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

/// Errors from encoding, decoding, or resolving a sent closure.
#[derive(Debug)]
pub enum CallError
{
  /// The argument/return type tag carried in the envelope doesn't match the
  /// tag embedded in the resolved decode function — caught by the target
  /// function itself, before it deserializes anything.
  ArgsOutputMismatch,

  /// The resolved function's own embedded call-site tag doesn't match the
  /// one in the envelope — caught by the target function itself, before it
  /// deserializes anything. In practice this means the two processes are not
  /// actually running the same binary, which this module assumes never happens.
  TypeMismatch { tag: u64 },

  /// Serialization of the closure or envelope failed.
  Encode(String),
  /// Deserialization of the closure or envelope failed.
  Decode(String)
}

impl std::fmt::Display for CallError
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}", self) }
}
impl std::error::Error for CallError {}

// =================================================================================================

/// Base load address of the binary containing this very function.
#[doc(hidden)]
pub fn moduleBase() -> usize
{
  let mut info: libc::Dl_info = unsafe{ std::mem::zeroed() };
  unsafe{ libc::dladdr(moduleBase as *const () as *const libc::c_void, &mut info) };
  info.dli_fbase as usize
}

/// Turns an absolute function pointer (in *this* process) into an offset.
#[doc(hidden)]
pub fn relativeOffsetOf(absoluteAddr: usize) -> usize
{
  absoluteAddr.wrapping_sub(moduleBase())
}

/// todo desc
fn resolveRelative(offset: usize) -> usize
{
  moduleBase().wrapping_add(offset)
}

/// Deterministic hash of a call-site source location.
#[doc(hidden)]
pub fn tagOf(sourceLocation: &str) -> u64
{
  let mut hasher: FxHasher = fxhash::FxHasher::default();
  sourceLocation.hash(&mut hasher);
  hasher.finish()
}

/// Deterministic hash of the argument/return [`Type`]s a callback was
/// declared with. Both sides of the wire compute it independently — the
/// sender in [`Sendable::encode`], the receiver inside the macro-generated
/// decode function — so an Args/Output mismatch is caught before the
/// captured state is deserialized.
///
/// Replaces the old `argsOutputTagOf::<Args, Output>()` (it hashed
/// `type_name`s of generic parameters that no longer exist at the decode
/// site now that `decode` is type-erased).
#[doc(hidden)]
pub fn typesTagOf(argTypes: &[Type], returnType: &Type) -> u64
{
  let mut hasher: FxHasher = FxHasher::default();
  argTypes.hash(&mut hasher);
  returnType.hash(&mut hasher);
  hasher.finish()
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

/// A concrete closure produced by [`callback!`], still on the originating side.
///
/// `State` is the tuple of captured variables, `Output` is the closure's
/// return type. Note what is *absent* from this signature: `Value`. The type
/// is therefore nameable — and [`Sendable::new`] callable — from
/// macro-generated code in foreign crates, where `Value` (`pub(crate)`)
/// cannot be referenced.
///
/// Call it directly with [`Sendable::call`] — no process boundary involved —
/// or [`Sendable::encode`] it to send across the zygote fork.
pub struct Sendable<State: Serialize + Send, Output: Primitive>
{
  /// Offset to the corresponding auto-generated decode function.
  relativeOffset: usize,

  /// Source code location hash used for target verification.
  siteTag: u64,

  /// todo desc
  pub(crate) argTypes: Vec<Type>,
  /// todo desc
  pub(crate) returnType: Type,

  /// The captured variables, as a tuple.
  state: State,

  /// Typed, same-process entry point into the closure body.
  typedFn: fn(&State, &DynamicList) -> Output
}

impl<State: Serialize + Send, Output: Primitive> Sendable<State, Output>
{
  #[doc(hidden)]
  pub fn new(
    relativeOffset: usize,
    siteTag: u64,
    argTypes: Vec<Type>,
    returnType: Type,
    state: State,
    typedFn: fn(&State, &DynamicList) -> Output
  ) -> Self
  {
    Self { relativeOffset, siteTag, argTypes, returnType, state, typedFn }
  }

  /// Calls the closure directly, in this process. Equivalent to calling the
  /// original closure — this never touches IPC or pointer resolution.
  pub fn call(&self, args: &DynamicList) -> Output
  {
    (self.typedFn)(&self.state, args)
  }

  /// Serializes everything needed to reconstruct and call this closure in
  /// the zygote clone.
  pub fn encode(&self) -> Result<Vec<u8>, CallError>
  {
    let bytes: Vec<u8> = bincode::serde::encode_to_vec(&self.state, bincode::config::standard())
      .map_err(|e| CallError::Encode(e.to_string()))?;
    let envelope: Envelope = Envelope {
      relativeOffset: self.relativeOffset,
      argsOutputTag: typesTagOf(&self.argTypes, &self.returnType),
      siteTag: self.siteTag,
      bytes
    };
    bincode::serde::encode_to_vec(&envelope, bincode::config::standard())
      .map_err(|e| CallError::Encode(e.to_string()))
  }
}

// =================================================================================================

/// The type-erased, dynamically callable form of a [`callback!`] closure —
/// what [`decode`] reconstructs inside the clone.
///
/// This is the public boundary of the otherwise `pub(crate)` dynamic world:
/// its constructor takes only nameable types (a state tuple + a typed fn
/// pointer), so macro-generated code in foreign crates can build it, while
/// actually *invoking* it (`ErasedCallable::call`, which does traffic in
/// `Value`) stays crate-internal. This is what allows `Value` to remain
/// `pub(crate)`.
pub struct ErasedCallable
{
  inner: Box<dyn Callable<DynamicList, Value>>
}

impl ErasedCallable
{
  /// Wraps a decoded capture-state tuple plus the macro-generated typed
  /// entry point into the erased, dispatcher-facing callable.
  #[doc(hidden)]
  pub fn fromStateAndFn<State: Send + 'static, Output: Primitive + 'static>(
    state: State,
    typedFn: fn(&State, &DynamicList) -> Output
  ) -> Self
  {
    Self { inner: Box::new(StateFnAdapter { state, typedFn }) }
  }

  /// Invokes the erased closure with dynamic arguments and returns the
  /// dynamic result. `pub(crate)`: only this crate's dispatcher (running
  /// inside the clone) ever needs it — `Value` is `pub(crate)`.
  pub(crate) fn call(&self, args: DynamicList) -> Value
  {
    self.inner.call(args)
  }
}

/// In-crate bridge from a macro-generated typed entry point to the dynamic
/// `Callable<CallbackArgs, Value>` object held by the dispatcher. The only
/// place where the two worlds meet.
struct StateFnAdapter<State: Send + 'static, Output: Primitive + 'static>
{
  state: State,
  typedFn: fn(&State, &DynamicList) -> Output
}

impl<State: Send + 'static, Output: Primitive + 'static> Callable<DynamicList, Value> for StateFnAdapter<State, Output>
{
  fn call(&self, args: DynamicList) -> Value
  {
    // The typed entry point returns the closure's concrete return type;
    // convert it to the dynamic form the C-side marshalling understands.
    <Output as Primitive>::toValue((self.typedFn)(&self.state, &args))
  }
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
        $crate::ffi::callback::ErasedCallable,
        $crate::ffi::callback::CallError
      >
      {
        let expectedSiteTag: u64 = $crate::ffi::callback::tagOf(
          concat!(file!(), ":", line!(), ":", column!())
        );
        if siteTag != expectedSiteTag {
          return ::std::result::Result::Err(
            $crate::ffi::callback::CallError::TypeMismatch { tag: siteTag }
          );
        }

        let expectedTypesTag: u64 = $crate::ffi::callback::typesTagOf(
          &[ $( <$argTy as $crate::ffi::types::primitive::Primitive>::TypeTag ),* ],
          &<$retTy as $crate::ffi::types::primitive::Primitive>::TypeTag
        );
        if argsOutputTag != expectedTypesTag {
          return ::std::result::Result::Err(
            $crate::ffi::callback::CallError::ArgsOutputMismatch
          );
        }

        let (state, _): (($($ty,)*), usize) =
          $crate::ffi::callback::__reexport::bincode::serde::decode_from_slice(
            bytes,
            $crate::ffi::callback::__reexport::bincode::config::standard()
          ).map_err(|e| $crate::ffi::callback::CallError::Decode(
            ::std::string::ToString::to_string(&e)
          ))?;

        ::std::result::Result::Ok(
          $crate::ffi::callback::ErasedCallable::fromStateAndFn(state, __callTyped)
        )
      }

      let siteTag: u64 = $crate::ffi::callback::tagOf(
        concat!(file!(), ":", line!(), ":", column!())
      );
      let relativeOffset: usize = $crate::ffi::callback::relativeOffsetOf(
        __callDecode as *const () as usize
      );
      let argTypes: ::std::vec::Vec<$crate::ffi::types::Type> =
        vec![ $( <$argTy as $crate::ffi::types::primitive::Primitive>::TypeTag ),* ];
      let returnType: $crate::ffi::types::Type =
        <$retTy as $crate::ffi::types::primitive::Primitive>::TypeTag;

      $scope.callback(
        $crate::ffi::callback::Sendable::new(
          relativeOffset,
          siteTag,
          argTypes,
          returnType,
          ($($name,)*),
          __callTyped
        )
      )
    }
  };
}

// =================================================================================================