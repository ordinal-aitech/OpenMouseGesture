use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

const APP_CONFIG_DIR_NAME: &str = "GestureHotkeyApp";
const DEFAULT_GROUP_ID: &str = "group-uncategorized";
const DEFAULT_GROUP_NAME: &str = "未分類";

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ValidationError {
    InvalidFormat(String),
    MissingRequiredField(String),
    InvalidValue(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ValidationError::InvalidFormat(msg) => write!(f, "invalid format: {}", msg),
            ValidationError::MissingRequiredField(field) => {
                write!(f, "missing required field: {}", field)
            }
            ValidationError::InvalidValue(msg) => write!(f, "invalid value: {}", msg),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureTemplate {
    pub name: String,
    pub points: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsBundle {
    pub formatVersion: u32,
    pub appName: String,
    pub exportedAt: String,
    pub config: Config,
    pub gestures: Vec<GestureTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionGroup {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Action {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub group_id: String,
    #[serde(default, skip_serializing)]
    pub group: String,
    #[serde(default = "default_trigger_type")]
    pub trigger_type: String,
    #[serde(default = "default_trigger_slot")]
    pub trigger_slot: String,
    #[serde(default)]
    pub gesture: String,
    #[serde(default)]
    pub wheel_trigger: Option<String>,
    #[serde(default)]
    pub action_type: String,
    #[serde(default)]
    pub keystroke: Option<String>,
    #[serde(default)]
    pub modifiers: Option<Vec<String>>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub operation: Option<String>,
    #[serde(default)]
    pub ignore_exe: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default = "default_trajectory")]
    pub trajectory: bool,
    #[serde(default)]
    pub ignore_exe: Vec<String>,
    #[serde(default = "default_trigger_button_right")]
    pub triggerA: String,
    #[serde(default = "default_trigger_button_middle")]
    pub triggerB: String,
    #[serde(default = "default_trigger_button_x1")]
    pub triggerC: String,
    #[serde(default = "default_trigger_a_color")]
    pub triggerAColor: String,
    #[serde(default = "default_trigger_b_color")]
    pub triggerBColor: String,
    #[serde(default = "default_trigger_c_color")]
    pub triggerCColor: String,
    #[serde(default)]
    pub groups: Vec<ActionGroup>,
    #[serde(default)]
    pub actions: Vec<Action>,
}

fn default_trajectory() -> bool {
    true
}

fn default_trigger_type() -> String {
    "gesture".to_string()
}

fn default_trigger_slot() -> String {
    "A".to_string()
}

fn default_trigger_button_right() -> String {
    "right".to_string()
}

fn default_trigger_button_middle() -> String {
    "middle".to_string()
}

fn default_trigger_button_x1() -> String {
    "x1".to_string()
}

fn default_trigger_a_color() -> String {
    "#FF4D4F".to_string()
}

fn default_trigger_b_color() -> String {
    "#4C8DFF".to_string()
}

fn default_trigger_c_color() -> String {
    "#22A06B".to_string()
}

fn default_group_id() -> String {
    DEFAULT_GROUP_ID.to_string()
}

fn default_group_name() -> String {
    DEFAULT_GROUP_NAME.to_string()
}

fn is_valid_trigger_button(value: &str) -> bool {
    matches!(value, "right" | "middle" | "x1" | "x2")
}

fn is_valid_trigger_slot(value: &str) -> bool {
    matches!(value, "A" | "B" | "C")
}

fn is_valid_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value.chars().skip(1).all(|c| c.is_ascii_hexdigit())
}

fn normalize_group_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default_group_name()
    } else {
        trimmed.to_string()
    }
}

fn normalize_group_id(value: &str) -> String {
    value.trim().to_string()
}

fn make_generated_group_id(counter: &mut usize, used_ids: &HashSet<String>) -> String {
    loop {
        let candidate = format!("group-{}", *counter);
        *counter += 1;
        if !used_ids.contains(&candidate) {
            return candidate;
        }
    }
}

fn register_group(
    mut group: ActionGroup,
    normalized_groups: &mut Vec<ActionGroup>,
    used_ids: &mut HashSet<String>,
    name_to_id: &mut HashMap<String, String>,
    generated_id_counter: &mut usize,
) -> String {
    group = group.normalized();
    if group.id.is_empty() || used_ids.contains(&group.id) {
        group.id = make_generated_group_id(generated_id_counter, used_ids);
    }

    if let Some(existing_id) = name_to_id.get(&group.name) {
        return existing_id.clone();
    }

    used_ids.insert(group.id.clone());
    name_to_id.insert(group.name.clone(), group.id.clone());
    normalized_groups.push(group.clone());
    group.id
}

impl Default for ActionGroup {
    fn default() -> Self {
        Self {
            id: default_group_id(),
            name: default_group_name(),
        }
    }
}

impl ActionGroup {
    fn normalized(mut self) -> Self {
        self.id = normalize_group_id(&self.id);
        self.name = normalize_group_name(&self.name);
        self
    }
}

impl Default for Action {
    fn default() -> Self {
        Self {
            name: String::new(),
            group_id: String::new(),
            group: String::new(),
            trigger_type: default_trigger_type(),
            trigger_slot: default_trigger_slot(),
            gesture: String::new(),
            wheel_trigger: None,
            action_type: String::new(),
            keystroke: None,
            modifiers: None,
            command: None,
            url: None,
            operation: None,
            ignore_exe: None,
        }
    }
}

impl Action {
    pub fn normalized(mut self) -> Self {
        if self.trigger_type.is_empty() {
            self.trigger_type = default_trigger_type();
        }

        if self.trigger_type == "gesture" && self.trigger_slot.is_empty() {
            self.trigger_slot = default_trigger_slot();
        }

        if !self.trigger_slot.is_empty() {
            self.trigger_slot = self.trigger_slot.to_uppercase();
        }

        if self.name.trim() == "past" {
            self.name = "paste".to_string();
        }

        self.group_id = normalize_group_id(&self.group_id);
        self.group = self.group.trim().to_string();

        self
    }

    fn validate(&self, known_group_ids: &HashSet<String>) -> Result<(), ValidationError> {
        if self.trigger_type != "gesture" && self.trigger_type != "wheel" {
            return Err(ValidationError::InvalidValue(format!(
                "trigger_type must be gesture or wheel: {}",
                self.trigger_type
            )));
        }

        if self.action_type != "keystroke"
            && self.action_type != "command"
            && self.action_type != "url"
            && self.action_type != "window_operation"
        {
            return Err(ValidationError::InvalidValue(format!(
                "unsupported action_type: {}",
                self.action_type
            )));
        }

        if self.group_id.trim().is_empty() {
            return Err(ValidationError::MissingRequiredField(
                "group_id".to_string(),
            ));
        }

        if !known_group_ids.contains(&self.group_id) {
            return Err(ValidationError::InvalidValue(format!(
                "unknown group_id: {}",
                self.group_id
            )));
        }

        if self.trigger_type == "gesture" {
            if self.gesture.is_empty() {
                return Err(ValidationError::MissingRequiredField("gesture".to_string()));
            }
            if !is_valid_trigger_slot(&self.trigger_slot) {
                return Err(ValidationError::InvalidValue(format!(
                    "invalid trigger_slot: {}",
                    self.trigger_slot
                )));
            }
        }

        if self.trigger_type == "wheel"
            && self
                .wheel_trigger
                .as_ref()
                .map_or(true, |s| s.trim().is_empty())
        {
            return Err(ValidationError::MissingRequiredField(
                "wheel_trigger".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            trajectory: default_trajectory(),
            ignore_exe: Vec::new(),
            triggerA: default_trigger_button_right(),
            triggerB: default_trigger_button_middle(),
            triggerC: default_trigger_button_x1(),
            triggerAColor: default_trigger_a_color(),
            triggerBColor: default_trigger_b_color(),
            triggerCColor: default_trigger_c_color(),
            groups: vec![ActionGroup::default()],
            actions: Vec::new(),
        }
    }
}

impl Config {
    pub fn normalized(mut self) -> Self {
        if !is_valid_trigger_button(&self.triggerA) {
            self.triggerA = default_trigger_button_right();
        }
        if !is_valid_trigger_button(&self.triggerB) {
            self.triggerB = default_trigger_button_middle();
        }
        if !is_valid_trigger_button(&self.triggerC) {
            self.triggerC = default_trigger_button_x1();
        }

        if !is_valid_hex_color(&self.triggerAColor) {
            self.triggerAColor = default_trigger_a_color();
        }
        if !is_valid_hex_color(&self.triggerBColor) {
            self.triggerBColor = default_trigger_b_color();
        }
        if !is_valid_hex_color(&self.triggerCColor) {
            self.triggerCColor = default_trigger_c_color();
        }

        self.actions = self
            .actions
            .into_iter()
            .map(|action| action.normalized())
            .collect();
        self.groups = normalize_groups_and_actions(self.groups, &mut self.actions);

        self
    }

    fn validate(&self) -> Result<(), ValidationError> {
        if !is_valid_trigger_button(&self.triggerA) {
            return Err(ValidationError::InvalidValue(format!(
                "invalid triggerA: {}",
                self.triggerA
            )));
        }
        if !is_valid_trigger_button(&self.triggerB) {
            return Err(ValidationError::InvalidValue(format!(
                "invalid triggerB: {}",
                self.triggerB
            )));
        }
        if !is_valid_trigger_button(&self.triggerC) {
            return Err(ValidationError::InvalidValue(format!(
                "invalid triggerC: {}",
                self.triggerC
            )));
        }

        if !is_valid_hex_color(&self.triggerAColor) {
            return Err(ValidationError::InvalidValue(format!(
                "invalid triggerAColor: {}",
                self.triggerAColor
            )));
        }
        if !is_valid_hex_color(&self.triggerBColor) {
            return Err(ValidationError::InvalidValue(format!(
                "invalid triggerBColor: {}",
                self.triggerBColor
            )));
        }
        if !is_valid_hex_color(&self.triggerCColor) {
            return Err(ValidationError::InvalidValue(format!(
                "invalid triggerCColor: {}",
                self.triggerCColor
            )));
        }

        if self.groups.is_empty() {
            return Err(ValidationError::MissingRequiredField("groups".to_string()));
        }

        let mut known_group_ids = HashSet::new();
        for (idx, group) in self.groups.iter().enumerate() {
            if group.id.trim().is_empty() {
                return Err(ValidationError::InvalidValue(format!(
                    "groups[{}] has empty id",
                    idx
                )));
            }
            if !known_group_ids.insert(group.id.clone()) {
                return Err(ValidationError::InvalidValue(format!(
                    "duplicate group id: {}",
                    group.id
                )));
            }
        }

        for (idx, action) in self.actions.iter().enumerate() {
            action
                .validate(&known_group_ids)
                .map_err(|e| ValidationError::InvalidValue(format!("actions[{}]: {}", idx, e)))?;
        }

        Ok(())
    }
}

fn normalize_groups_and_actions(
    groups: Vec<ActionGroup>,
    actions: &mut [Action],
) -> Vec<ActionGroup> {
    let mut normalized_groups = Vec::new();
    let mut used_ids = HashSet::new();
    let mut name_to_id: HashMap<String, String> = HashMap::new();
    let mut generated_id_counter = 1usize;

    for group in groups {
        register_group(
            group,
            &mut normalized_groups,
            &mut used_ids,
            &mut name_to_id,
            &mut generated_id_counter,
        );
    }

    if !used_ids.contains(DEFAULT_GROUP_ID) {
        register_group(
            ActionGroup {
                id: DEFAULT_GROUP_ID.to_string(),
                name: DEFAULT_GROUP_NAME.to_string(),
            },
            &mut normalized_groups,
            &mut used_ids,
            &mut name_to_id,
            &mut generated_id_counter,
        );
    }

    for action in actions.iter_mut() {
        let legacy_group_name = if action.group.trim().is_empty() {
            None
        } else {
            Some(normalize_group_name(&action.group))
        };

        let current_group_id = normalize_group_id(&action.group_id);
        let resolved_group_id =
            if !current_group_id.is_empty() && used_ids.contains(&current_group_id) {
                current_group_id
            } else if let Some(group_name) = legacy_group_name {
                if let Some(existing_id) = name_to_id.get(&group_name) {
                    existing_id.clone()
                } else {
                    register_group(
                        ActionGroup {
                            id: String::new(),
                            name: group_name,
                        },
                        &mut normalized_groups,
                        &mut used_ids,
                        &mut name_to_id,
                        &mut generated_id_counter,
                    )
                }
            } else {
                DEFAULT_GROUP_ID.to_string()
            };

        action.group_id = resolved_group_id;
        action.group.clear();
    }

    normalized_groups
}

impl GestureTemplate {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.name.is_empty() {
            return Err(ValidationError::MissingRequiredField("name".to_string()));
        }
        if self.points.is_empty() {
            return Err(ValidationError::MissingRequiredField("points".to_string()));
        }
        Ok(())
    }
}

pub struct ConfigManager {
    config_dir: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Result<Self, String> {
        let config_dir = if cfg!(debug_assertions) {
            let manifest_dir = env!("CARGO_MANIFEST_DIR");
            PathBuf::from(manifest_dir)
                .parent()
                .ok_or("Failed to get project root directory")?
                .join("config")
        } else {
            dirs::config_dir()
                .ok_or("Failed to resolve user config directory")?
                .join(APP_CONFIG_DIR_NAME)
        };

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        if !cfg!(debug_assertions) {
            migrate_legacy_release_files(&config_dir)?;
        }

        Ok(ConfigManager { config_dir })
    }

    pub fn load_gestures(&self) -> Result<Vec<GestureTemplate>, String> {
        let path = self.config_dir.join("gestures.json");

        if !path.exists() {
            let default_gestures = include_str!("../../config/default-gestures.json");
            let gestures: Vec<GestureTemplate> = serde_json::from_str(default_gestures)
                .map_err(|e| format!("Failed to parse default gestures: {}", e))?;
            self.save_gestures(&gestures)?;
            return Ok(gestures);
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read gestures.json: {}", e))?;

        let gestures: Vec<GestureTemplate> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse gestures.json: {}", e))?;

        for (idx, gesture) in gestures.iter().enumerate() {
            if let Err(e) = gesture.validate() {
                return Err(format!(
                    "gestures.json validation error at index {}: {}",
                    idx, e
                ));
            }
        }

        Ok(gestures)
    }

    pub fn save_gestures(&self, gestures: &[GestureTemplate]) -> Result<(), String> {
        for (idx, gesture) in gestures.iter().enumerate() {
            if let Err(e) = gesture.validate() {
                return Err(format!(
                    "gestures.json validation error at index {}: {}",
                    idx, e
                ));
            }
        }

        let path = self.config_dir.join("gestures.json");
        let content = serde_json::to_string_pretty(gestures)
            .map_err(|e| format!("Failed to serialize gestures: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Failed to write gestures.json: {}", e))?;

        Ok(())
    }

    pub fn load_config(&self) -> Result<Config, String> {
        let path = self.config_dir.join("config.json");

        if !path.exists() {
            let default_config = include_str!("../../config/default-config.json");
            let config: Config = serde_json::from_str(default_config)
                .map_err(|e| format!("Failed to parse default config: {}", e))?;
            let config = config.normalized();
            self.save_config(&config)?;
            return Ok(config);
        }

        let content =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read config.json: {}", e))?;

        let parsed: Config = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config.json: {}", e))?;
        let normalized = parsed.clone().normalized();

        if let Err(e) = normalized.validate() {
            return Err(format!("config.json validation error: {}", e));
        }

        if normalized != parsed {
            self.save_config(&normalized)?;
        }

        Ok(normalized)
    }

    pub fn save_config(&self, config: &Config) -> Result<(), String> {
        let normalized = config.clone().normalized();
        if let Err(e) = normalized.validate() {
            return Err(format!("config.json validation error: {}", e));
        }

        let path = self.config_dir.join("config.json");
        let content = serde_json::to_string_pretty(&normalized)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Failed to write config.json: {}", e))?;

        Ok(())
    }

    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    pub fn build_settings_bundle(&self) -> Result<SettingsBundle, String> {
        let config = self.load_config()?;
        let gestures = self.load_gestures()?;

        Ok(SettingsBundle {
            formatVersion: 1,
            appName: "GestureHotkeyApp".to_string(),
            exportedAt: chrono_like_timestamp(),
            config,
            gestures,
        })
    }

    pub fn import_settings_bundle(&self, bundle: SettingsBundle) -> Result<(), String> {
        if bundle.formatVersion == 0 {
            return Err("Unsupported settings bundle formatVersion".to_string());
        }

        for (idx, gesture) in bundle.gestures.iter().enumerate() {
            if let Err(e) = gesture.validate() {
                return Err(format!(
                    "settings bundle gestures[{}] validation error: {}",
                    idx, e
                ));
            }
        }

        let normalized_config = bundle.config.normalized();
        if let Err(e) = normalized_config.validate() {
            return Err(format!("settings bundle config validation error: {}", e));
        }

        self.save_gestures(&bundle.gestures)?;
        self.save_config(&normalized_config)?;
        Ok(())
    }
}

fn chrono_like_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}", duration.as_secs()),
        Err(_) => "0".to_string(),
    }
}

fn migrate_legacy_release_files(target_dir: &PathBuf) -> Result<(), String> {
    let legacy_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path for migration: {}", e))?
        .parent()
        .ok_or("Failed to get executable directory for migration")?
        .to_path_buf();

    if legacy_dir == *target_dir {
        return Ok(());
    }

    for file_name in ["config.json", "gestures.json"] {
        let source = legacy_dir.join(file_name);
        let target = target_dir.join(file_name);

        if source.exists() && !target.exists() {
            fs::copy(&source, &target).map_err(|e| {
                format!(
                    "Failed to migrate {} from {} to {}: {}",
                    file_name,
                    source.display(),
                    target.display(),
                    e
                )
            })?;
        }
    }

    Ok(())
}
