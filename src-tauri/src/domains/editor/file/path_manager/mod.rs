pub struct PathManager;
impl PathManager { pub fn resolve(base: &str, path: &std::path::Path) -> String { format!("{}/{}", base, path.display()) } pub fn normalize(path: &str) -> String { path.to_string() } }
