//! Skill store.
//!
//! Skills are prompt packages and matching rules, not executable capabilities.
//! They stay in a Skills-domain store and are activated by Agent/Tool pipeline
//! logic that executes real tools through Kernel Registry / Policy / Pipeline.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

use super::{SkillDefinition, SkillMode, SkillSource};

/// Skills-domain store for `SkillDefinition` DTOs.
///
/// This is deliberately not a Kernel Registry: Skill files do not expose
/// lifecycle-managed executable capability implementations. They provide
/// prompts, tool allowlists and matching metadata consumed by Agent pipelines.
pub struct SkillStore {
    skills: HashMap<String, Arc<SkillDefinition>>,
}

impl SkillStore {
    pub fn new() -> Self {
        tracing::debug!("Creating SkillStore");
        Self {
            skills: HashMap::new(),
        }
    }

    pub fn upsert(&mut self, skill: SkillDefinition) -> Result<()> {
        let id = skill.id.clone();
        tracing::debug!(skill_id = %id, name = %skill.name, "Upserting skill");
        self.skills.insert(id, Arc::new(skill));
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<Arc<SkillDefinition>> {
        self.skills.get(id).cloned()
    }

    pub fn find_by_trigger(&self, trigger: &str) -> Option<Arc<SkillDefinition>> {
        self.list_enabled()
            .into_iter()
            .find(|skill| skill.trigger.as_deref() == Some(trigger))
    }

    pub fn list(&self) -> Vec<Arc<SkillDefinition>> {
        self.skills.values().cloned().collect()
    }

    pub fn list_enabled(&self) -> Vec<Arc<SkillDefinition>> {
        self.skills
            .values()
            .filter(|skill| skill.enabled)
            .cloned()
            .collect()
    }

    pub fn list_by_mode(&self, mode: &SkillMode) -> Vec<Arc<SkillDefinition>> {
        self.list()
            .into_iter()
            .filter(|skill| &skill.mode == mode)
            .collect()
    }

    pub fn list_by_source(&self, source: &SkillSource) -> Vec<Arc<SkillDefinition>> {
        self.list()
            .into_iter()
            .filter(|skill| &skill.source == source)
            .collect()
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<()> {
        let Some(skill) = self.skills.get(id) else {
            return Err(anyhow::anyhow!("Skill not found: {}", id));
        };
        let mut updated = skill.as_ref().clone();
        updated.enabled = enabled;
        self.skills.insert(id.to_string(), Arc::new(updated));

        tracing::info!(skill_id = %id, enabled = enabled, "Skill enabled state changed");
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Result<()> {
        if self.skills.remove(id).is_none() {
            return Err(anyhow::anyhow!("Skill not found: {}", id));
        }
        tracing::info!(skill_id = %id, "Skill removed from store");
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.skills.len()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.skills.contains_key(id)
    }
}

impl Default for SkillStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SkillMode, SkillSource};
    use super::*;
    use std::path::PathBuf;

    fn create_test_skill(id: &str, name: &str, trigger: Option<&str>) -> SkillDefinition {
        SkillDefinition {
            id: id.to_string(),
            name: name.to_string(),
            description: format!("Test skill: {}", name),
            mode: SkillMode::Standard,
            version: "1.0.0".to_string(),
            source: SkillSource::Builtin,
            file_path: PathBuf::from("test"),
            trigger: trigger.map(|t| t.to_string()),
            tools_whitelist: Vec::new(),
            parameters: Vec::new(),
            steps: Vec::new(),
            content: "test content".to_string(),
            enabled: true,
            source_url: None,
            installed_at: None,
            author: None,
            tags: None,
            min_navis_version: None,
        }
    }

    #[test]
    fn test_upsert_and_get() {
        let mut store = SkillStore::new();
        let skill = create_test_skill("test:review", "review", Some("/review"));
        store.upsert(skill).unwrap();

        assert!(store.contains("test:review"));
        let retrieved = store.get("test:review").unwrap();
        assert_eq!(retrieved.name, "review");
    }

    #[test]
    fn test_get_nonexistent() {
        let store = SkillStore::new();
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn test_find_by_trigger() {
        let mut store = SkillStore::new();
        store
            .upsert(create_test_skill("a:review", "review", Some("/review")))
            .unwrap();
        store
            .upsert(create_test_skill("a:commit", "commit", Some("/commit")))
            .unwrap();

        let found = store.find_by_trigger("/review");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "review");

        assert!(store.find_by_trigger("/nonexistent").is_none());
    }

    #[test]
    fn test_find_by_trigger_disabled() {
        let mut store = SkillStore::new();
        let mut skill = create_test_skill("a:review", "review", Some("/review"));
        skill.enabled = false;
        store.upsert(skill).unwrap();

        assert!(store.find_by_trigger("/review").is_none());
    }

    #[test]
    fn test_list() {
        let mut store = SkillStore::new();
        store
            .upsert(create_test_skill("a", "skill-a", None))
            .unwrap();
        store
            .upsert(create_test_skill("b", "skill-b", None))
            .unwrap();

        assert_eq!(store.list().len(), 2);
    }

    #[test]
    fn test_list_enabled() {
        let mut store = SkillStore::new();
        store
            .upsert(create_test_skill("a", "skill-a", None))
            .unwrap();
        let mut disabled = create_test_skill("b", "skill-b", None);
        disabled.enabled = false;
        store.upsert(disabled).unwrap();

        assert_eq!(store.list_enabled().len(), 1);
    }

    #[test]
    fn test_set_enabled() {
        let mut store = SkillStore::new();
        store
            .upsert(create_test_skill("a", "skill-a", None))
            .unwrap();

        assert!(store.get("a").unwrap().enabled);

        store.set_enabled("a", false).unwrap();
        assert!(!store.get("a").unwrap().enabled);

        store.set_enabled("a", true).unwrap();
        assert!(store.get("a").unwrap().enabled);
    }

    #[test]
    fn test_set_enabled_nonexistent() {
        let mut store = SkillStore::new();
        let result = store.set_enabled("nonexistent", true);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove() {
        let mut store = SkillStore::new();
        store
            .upsert(create_test_skill("a", "skill-a", None))
            .unwrap();

        assert!(store.contains("a"));
        store.remove("a").unwrap();
        assert!(!store.contains("a"));
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut store = SkillStore::new();
        let result = store.remove("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_count() {
        let mut store = SkillStore::new();
        assert_eq!(store.count(), 0);

        store
            .upsert(create_test_skill("a", "skill-a", None))
            .unwrap();
        assert_eq!(store.count(), 1);

        store
            .upsert(create_test_skill("b", "skill-b", None))
            .unwrap();
        assert_eq!(store.count(), 2);
    }

    #[test]
    fn test_list_by_mode() {
        let mut store = SkillStore::new();
        let mut enhanced = create_test_skill("a", "enhanced-skill", None);
        enhanced.mode = SkillMode::Enhanced;
        store.upsert(enhanced).unwrap();
        store
            .upsert(create_test_skill("b", "standard-skill", None))
            .unwrap();

        let standards = store.list_by_mode(&SkillMode::Standard);
        assert_eq!(standards.len(), 1);
        assert_eq!(standards[0].name, "standard-skill");

        let enhanced_list = store.list_by_mode(&SkillMode::Enhanced);
        assert_eq!(enhanced_list.len(), 1);
        assert_eq!(enhanced_list[0].name, "enhanced-skill");
    }

    #[test]
    fn test_list_by_source() {
        let mut store = SkillStore::new();
        let mut user_skill = create_test_skill("a", "user-skill", None);
        user_skill.source = SkillSource::User;
        store.upsert(user_skill).unwrap();
        store
            .upsert(create_test_skill("b", "builtin-skill", None))
            .unwrap();

        let user_list = store.list_by_source(&SkillSource::User);
        assert_eq!(user_list.len(), 1);

        let builtin_list = store.list_by_source(&SkillSource::Builtin);
        assert_eq!(builtin_list.len(), 1);
    }

    #[test]
    fn test_upsert_replaces_skill() {
        let mut store = SkillStore::new();
        store
            .upsert(create_test_skill("a", "skill-a", None))
            .unwrap();
        store.set_enabled("a", false).unwrap();

        let mut updated = create_test_skill("a", "skill-a-updated", None);
        updated.enabled = false;
        store.upsert(updated).unwrap();

        let skill = store.get("a").unwrap();
        assert_eq!(skill.name, "skill-a-updated");
        assert!(!skill.enabled);
        assert!(store.list_enabled().is_empty());
    }
}
