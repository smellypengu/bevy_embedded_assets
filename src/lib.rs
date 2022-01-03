#![doc = include_str!("../README.md")]
#![warn(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    missing_docs
)]

use std::path::{Path, PathBuf};

use bevy::{
    asset::{AssetIo, AssetIoError},
    utils::HashMap,
};

mod plugin;
pub use plugin::EmbeddedAssetPlugin;

include!(concat!(env!("OUT_DIR"), "/include_all_assets.rs"));

/// An [`HashMap`] associating file paths to their content, that can be used as an [`AssetIo`]
pub struct EmbeddedAssetIo {
    loaded: HashMap<&'static Path, &'static [u8]>,
}

impl std::fmt::Debug for EmbeddedAssetIo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddedAssetIo").finish_non_exhaustive()
    }
}

impl Default for EmbeddedAssetIo {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddedAssetIo {
    /// Create an empty [`EmbeddedAssetIo`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            loaded: HashMap::default(),
        }
    }

    /// Create an [`EmbeddedAssetIo`] loaded with all the assets found by the build script.
    #[must_use]
    pub fn preloaded() -> Self {
        let mut new = Self {
            loaded: HashMap::default(),
        };
        include_all_assets(&mut new);
        new
    }

    /// Add an asset to this [`EmbeddedAssetIo`].
    pub fn add_asset(&mut self, path: &'static Path, data: &'static [u8]) {
        self.loaded.insert(path, data);
    }

    /// ZUT
    pub fn load_path_sync(&self, path: &Path) -> Result<Vec<u8>, AssetIoError> {
        self.loaded
            .get(path)
            .map(|b| b.to_vec())
            .ok_or_else(|| bevy::asset::AssetIoError::NotFound(path.to_path_buf()))
    }
}

impl AssetIo for EmbeddedAssetIo {
    fn load_path<'a>(
        &'a self,
        path: &'a Path,
    ) -> bevy::utils::BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        Box::pin(async move { self.load_path_sync(path) })
    }

    #[allow(clippy::needless_collect)]
    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        if self.is_directory(path) {
            let paths: Vec<_> = self
                .loaded
                .keys()
                .filter(|loaded_path| loaded_path.starts_with(path))
                .map(|t| t.to_path_buf())
                .collect();
            Ok(Box::new(paths.into_iter()))
        } else {
            Err(AssetIoError::Io(std::io::ErrorKind::NotFound.into()))
        }
    }

    fn is_directory(&self, path: &Path) -> bool {
        let as_folder = path.join("");
        self.loaded
            .keys()
            .any(|loaded_path| loaded_path.starts_with(&as_folder) && loaded_path != &path)
    }

    fn watch_path_for_changes(&self, _path: &Path) -> Result<(), AssetIoError> {
        Ok(())
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use bevy::asset::AssetIo;

    use crate::EmbeddedAssetIo;

    #[test]
    fn load_path() {
        let mut embedded = EmbeddedAssetIo::new();
        embedded.add_asset(&Path::new("asset.png"), &[1, 2, 3]);
        embedded.add_asset(&Path::new("other_asset.png"), &[4, 5, 6]);

        assert!(embedded.load_path_sync(&Path::new("asset.png")).is_ok());
        assert_eq!(
            embedded.load_path_sync(&Path::new("asset.png")).unwrap(),
            [1, 2, 3]
        );
        assert_eq!(
            embedded
                .load_path_sync(&Path::new("other_asset.png"))
                .unwrap(),
            [4, 5, 6]
        );
        assert!(embedded.load_path_sync(&Path::new("asset")).is_err());
        assert!(embedded.load_path_sync(&Path::new("other")).is_err());
    }

    #[test]
    fn is_directory() {
        let mut embedded = EmbeddedAssetIo::new();
        embedded.add_asset(&Path::new("asset.png"), &[]);
        embedded.add_asset(&Path::new("directory/asset.png"), &[]);

        assert!(!embedded.is_directory(&Path::new("asset.png")));
        assert!(!embedded.is_directory(&Path::new("asset")));
        assert!(embedded.is_directory(&Path::new("directory")));
        assert!(embedded.is_directory(&Path::new("directory/")));
        assert!(!embedded.is_directory(&Path::new("directory/asset")));
    }

    #[test]
    fn read_directory() {
        let mut embedded = EmbeddedAssetIo::new();
        embedded.add_asset(&Path::new("asset.png"), &[]);
        embedded.add_asset(&Path::new("directory/asset.png"), &[]);
        embedded.add_asset(&Path::new("directory/asset2.png"), &[]);

        assert!(embedded.read_directory(&Path::new("asset.png")).is_err());
        assert!(embedded.read_directory(&Path::new("directory")).is_ok());
        let mut list = embedded
            .read_directory(&Path::new("directory"))
            .unwrap()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        list.sort();
        assert_eq!(list, vec!["directory/asset.png", "directory/asset2.png"]);
    }
}