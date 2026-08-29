//! The JAR classpath seam behind `Class.getResourceAsStream` — a host boundary.
//!
//! `AssetCache.readResource` (and the `.mph`/`.mpd` loaders it feeds) reach the
//! surviving builds through `new Object().getClass().getResourceAsStream(path)`,
//! i.e. the MIDlet JAR's classpath. That classpath is a device/host fact, not
//! game logic, so it lives behind this seam exactly like `j2me-me`'s
//! [`ImageResources`](j2me_me::ImageResources) seam for `Image.createImage(String)`:
//! the host (a test, a runner) populates the bank from the baseline JAR, and the
//! transliterated `readResource` reads from it.
//!
//! A resource name is the JAR-absolute path the game passes (`"/img/logo.mph"`);
//! zip entries are stored without the leading slash (`"img/logo.mph"`), so
//! [`ResourceBank::get`] strips one leading `/` before matching — reproducing
//! `getResourceAsStream`'s absolute-name resolution. An absent name resolves to
//! `None`, which the game reads as `getResourceAsStream` returning `null`.

/// The classpath resource store. Bytes are Java `byte[]` (`Vec<i8>`), the shape
/// `readResource` reads.
#[derive(Debug, Default)]
pub struct ResourceBank {
    entries: Vec<(String, Vec<i8>)>,
}

impl ResourceBank {
    /// An empty bank (no resources visible on the classpath).
    pub fn new() -> Self {
        ResourceBank {
            entries: Vec::new(),
        }
    }

    /// Registers `bytes` under the JAR entry `name` (stored without a leading
    /// slash, e.g. `"img/logo.mph"`).
    pub fn insert(&mut self, name: impl Into<String>, bytes: Vec<i8>) {
        self.entries.push((name.into(), bytes));
    }

    /// Resolves a game-side resource path (`getResourceAsStream(path)`): one
    /// leading `/` is stripped to form the JAR entry name; `None` mirrors a
    /// `null` stream.
    pub fn get(&self, path: &str) -> Option<&[i8]> {
        let name = path.strip_prefix('/').unwrap_or(path);
        self.entries
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| b.as_slice())
    }
}
