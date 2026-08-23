use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::OnceLock;
// =================================================================================================

/// todo desc
#[derive(Default)]
pub struct PathResolver
{
  /// todo desc
  dirs: Vec<PathBuf>
}

impl PathResolver
{
  /// todo desc
  pub fn addPath(&mut self, path: impl Into<PathBuf>) -> ()
  {
    self.dirs.push(path.into());
  }

  /// todo desc
  pub fn resolve(&self, name: &str) -> Option<String>
  {
    if name.contains('/') { return None; }
    self.dirs.iter()
      .map(|dir| dir.join(name))
      .find(|p| p.exists())
      .map(|p| p.to_string_lossy().into_owned())
  }
}

// =================================================================================================

/// todo desc
static GlobalPaths: OnceLock<RwLock<PathResolver>> = OnceLock::new();

/// todo desc
pub fn addGlobalSearchPath(path: impl Into<PathBuf>) -> ()
{
  GlobalPaths.get_or_init(|| RwLock::new(PathResolver::default()))
    .write()
    .addPath(path);
}

/// todo desc
pub(super) fn resolveGlobal(name: &str) -> Option<String>
{
  GlobalPaths.get()?
    .read()
    .resolve(name)
}

// =================================================================================================

// todo tests простые. реальный path проверяется в example