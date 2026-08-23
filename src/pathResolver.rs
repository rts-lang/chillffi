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

#[cfg(test)]
mod tests
{
  use super::*;
  use std::fs::File;
  use std::env::temp_dir;
  // ===============================================================================================

  /// todo desc
  #[test]
  fn ignoresPathWithSlash() -> ()
  {
    let resolver: PathResolver = PathResolver::default();
    assert_eq!(resolver.resolve("foo/bar.so"), None);
  }

  /// todo desc
  #[test]
  fn returnsNoneWhenNotFound() -> ()
  {
    let mut resolver: PathResolver = PathResolver::default();
    resolver.addPath("/nonexistent/dir");
    assert_eq!(resolver.resolve("libNope.so"), None);
  }

  /// todo desc
  #[test]
  fn findsExistingFile() -> ()
  {
    let dir: PathBuf = temp_dir();
    let fileName: &str = "chillffiTestResolve.so";
    File::create(dir.join(fileName)).unwrap();

    let mut resolver: PathResolver = PathResolver::default();
    resolver.addPath(&dir);

    let resolved: String = resolver.resolve(fileName).expect("should find file");
    assert!(resolved.ends_with(fileName));

    std::fs::remove_file(dir.join(fileName)).unwrap();
  }

  /// todo desc
  #[test]
  fn globalRoundtrip() -> ()
  {
    let dir: PathBuf = temp_dir();
    let fileName: &str = "chillffiTestGlobal.so";
    File::create(dir.join(fileName)).unwrap();

    addGlobalSearchPath(&dir);
    let resolved: String = resolveGlobal(fileName).expect("should find via global");
    assert!(resolved.ends_with(fileName));

    std::fs::remove_file(dir.join(fileName)).unwrap();
  }

  // ===============================================================================================
}

// =================================================================================================