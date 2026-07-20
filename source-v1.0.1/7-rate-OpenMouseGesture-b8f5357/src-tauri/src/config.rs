use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

const APP_CONFIG_DIR_NAME: &str = "GestureHotkeyApp";
const DEFAULT_GROUP_ID: &str = "group-uncategorized";
const UNASSIGNED_TRIGGER: &str = "";
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
    #[serde(default)]
    pub text: Option<String>,
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
    "mouse:right".to_string()
}

fn default_trigger_button_middle() -> String {
    "mouse:middle".to_string()
}

fn default_trigger_button_x1() -> String {
    "mouse:x1".to_string()
}

/// 左クリックは通常操作と競合し操作不能ロックアウトを招くため、
/// トリガーとして一切受け付けない（"left"/"mouse:left" は意図的に非対応）。
fn normalize_mouse_trigger(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "right" | "mouse:right" => Some("right"),
        "middle" | "mouse:middle" => Some("middle"),
        "x1" | "mouse:x1" => Some("x1"),
        "x2" | "mouse:x2" => Some("x2"),
        _ => None,
    }
}

fn is_left_mouse_trigger(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "left" | "mouse:left"
    )
}

fn normalize_trigger_modifier(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Some("Ctrl"),
        "alt" => Some("Alt"),
        "shift" => Some("Shift"),
        _ => None,
    }
}

fn normalize_trigger_modifiers(values: &[&str]) -> Option<Vec<String>> {
    let normalized: Vec<&'static str> = values
        .iter()
        .filter_map(|value| normalize_trigger_modifier(value))
        .collect();

    if normalized.len() != values.len() {
        return None;
    }

    ordered_modifier_values(normalized)
}

fn ordered_modifier_values(values: Vec<&'static str>) -> Option<Vec<String>> {
    Some(
        ["Ctrl", "Alt", "Shift"]
            .into_iter()
            .filter(|modifier| values.iter().any(|value| value == modifier))
            .map(|modifier| modifier.to_string())
            .collect(),
    )
}

fn format_keyboard_trigger(modifiers: &[String], code: &str) -> String {
    if modifiers.is_empty() {
        format!("key:{}", code)
    } else {
        format!("key:{}+{}", modifiers.join("+"), code)
    }
}

#[allow(dead_code)]
pub fn display_key_for_code(code: &str) -> Option<String> {
    if code.starts_with("Key") && code.len() == 4 {
        return Some(code[3..4].to_string());
    }

    if code.starts_with("Digit") && code.len() == 6 {
        return Some(code[5..6].to_string());
    }

    if let Some(suffix) = code.strip_prefix("F") {
        if suffix.parse::<u8>().ok().filter(|value| *value >= 1 && *value <= 24).is_some() {
            return Some(code.to_string());
        }
    }

    if code.starts_with("Numpad") && code.len() == 7 {
        let digit = &code[6..7];
        if digit.chars().all(|c| c.is_ascii_digit()) {
            return Some(format!("Num {}", digit));
        }
    }

    match code {
        "ArrowDown" => Some("Down".to_string()),
        "ArrowLeft" => Some("Left".to_string()),
        "ArrowRight" => Some("Right".to_string()),
        "ArrowUp" => Some("Up".to_string()),
        "Backspace" => Some("Backspace".to_string()),
        "CapsLock" => Some("CapsLock".to_string()),
        "Delete" => Some("Delete".to_string()),
        "End" => Some("End".to_string()),
        "Enter" => Some("Enter".to_string()),
        "Equal" => Some("=".to_string()),
        "Escape" => Some("Escape".to_string()),
        "Home" => Some("Home".to_string()),
        "Insert" => Some("Insert".to_string()),
        "Minus" => Some("-".to_string()),
        "NumpadAdd" => Some("Num +".to_string()),
        "NumpadDecimal" => Some("Num .".to_string()),
        "NumpadDivide" => Some("Num /".to_string()),
        "NumpadEnter" => Some("Num Enter".to_string()),
        "NumpadMultiply" => Some("Num *".to_string()),
        "NumpadSubtract" => Some("Num -".to_string()),
        "PageDown" => Some("PageDown".to_string()),
        "PageUp" => Some("PageUp".to_string()),
        "Pause" => Some("Pause".to_string()),
        "Period" => Some(".".to_string()),
        "PrintScreen" => Some("PrintScreen".to_string()),
        "ScrollLock" => Some("ScrollLock".to_string()),
        "Semicolon" => Some(";".to_string()),
        "Slash" => Some("/".to_string()),
        "Space" => Some("Space".to_string()),
        "Tab" => Some("Tab".to_string()),
        _ => None,
    }
}

pub fn keyboard_code_to_vk(code: &str) -> Option<u16> {
    if code.starts_with("Key") && code.len() == 4 {
        let c = code.as_bytes()[3];
        if c.is_ascii_uppercase() {
            return Some(c as u16);
        }
    }

    if code.starts_with("Digit") && code.len() == 6 {
        let c = code.as_bytes()[5];
        if c.is_ascii_digit() {
            return Some(c as u16);
        }
    }

    if let Some(suffix) = code.strip_prefix("F") {
        if let Ok(value) = suffix.parse::<u16>() {
            if (1..=24).contains(&value) {
                return Some(0x70 + value - 1);
            }
        }
    }

    if code.starts_with("Numpad") && code.len() == 7 {
        let c = code.as_bytes()[6];
        if c.is_ascii_digit() {
            return Some(0x60 + (c - b'0') as u16);
        }
    }

    match code {
        "ArrowLeft" => Some(0x25),
        "ArrowUp" => Some(0x26),
        "ArrowRight" => Some(0x27),
        "ArrowDown" => Some(0x28),
        "Backspace" => Some(0x08),
        "Tab" => Some(0x09),
        "Enter" | "NumpadEnter" => Some(0x0D),
        "Pause" => Some(0x13),
        "CapsLock" => Some(0x14),
        "Escape" => Some(0x1B),
        "Space" => Some(0x20),
        "PageUp" => Some(0x21),
        "PageDown" => Some(0x22),
        "End" => Some(0x23),
        "Home" => Some(0x24),
        "Insert" => Some(0x2D),
        "Delete" => Some(0x2E),
        "PrintScreen" => Some(0x2C),
        "ScrollLock" => Some(0x91),
        "Minus" => Some(0xBD),
        "Equal" => Some(0xBB),
        "Semicolon" => Some(0xBA),
        "Slash" => Some(0xBF),
        "Period" => Some(0xBE),
        "NumpadMultiply" => Some(0x6A),
        "NumpadAdd" => Some(0x6B),
        "NumpadSubtract" => Some(0x6D),
        "NumpadDecimal" => Some(0x6E),
        "NumpadDivide" => Some(0x6F),
        _ => None,
    }
}

pub fn parse_keyboard_trigger(value: &str) -> Option<(Vec<String>, String)> {
    let payload = value.trim().strip_prefix("key:")?;
    let parts: Vec<&str> = payload.split('+').map(|part| part.trim()).filter(|part| !part.is_empty()).collect();
    let (code, modifiers) = parts.split_last()?;
    let normalized_modifiers = normalize_trigger_modifiers(modifiers)?;
    if keyboard_code_to_vk(code).is_none() {
        return None;
    }
    Some((normalized_modifiers, (*code).to_string()))
}

pub fn normalize_trigger_binding(value: &str, default_value: &str) -> String {
    if value.trim().is_empty() {
        return UNASSIGNED_TRIGGER.to_string();
    }

    if is_left_mouse_trigger(value) {
        return UNASSIGNED_TRIGGER.to_string();
    }

    if let Some(button) = normalize_mouse_trigger(value) {
        return format!("mouse:{}", button);
    }

    if let Some((modifiers, code)) = parse_keyboard_trigger(value) {
        return format_keyboard_trigger(&modifiers, &code);
    }

    if let Some(button) = normalize_mouse_trigger(default_value) {
        return format!("mouse:{}", button);
    }

    default_value.to_string()
}

fn is_valid_trigger_binding(value: &str) -> bool {
    value.trim().is_empty()
        || normalize_mouse_trigger(value).is_some()
        || parse_keyboard_trigger(value).is_some()
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
            text: None,
        }
    }
}

impl Action {
    pub fn normalized(mut self) -> Self {
        if self.trigger_type.is_empty() {
            self.trigger_type = default_trigger_type();
        }

        if (self.trigger_type == "gesture" || self.trigger_type == "wheel") && self.trigger_slot.is_empty() {
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
            && self.action_type != "text"
        {
            return Err(ValidationError::InvalidValue(format!(
                "unsupported action_type: {}",
                self.action_type
            )));
        }

        if self.action_type == "text"
            && self.text.as_ref().map_or(true, |s| s.trim().is_empty())
        {
            return Err(ValidationError::MissingRequiredField("text".to_string()));
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

        if self.trigger_type == "wheel" {
            if self
                .wheel_trigger
                .as_ref()
                .map_or(true, |s| s.trim().is_empty())
            {
                return Err(ValidationError::MissingRequiredField(
                    "wheel_trigger".to_string(),
                ));
            }
            if !is_valid_trigger_slot(&self.trigger_slot) {
                return Err(ValidationError::InvalidValue(format!(
                    "invalid trigger_slot: {}",
                    self.trigger_slot
                )));
            }
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
        self.triggerA = normalize_trigger_binding(&self.triggerA, &default_trigger_button_right());
        self.triggerB = normalize_trigger_binding(&self.triggerB, &default_trigger_button_middle());
        self.triggerC = normalize_trigger_binding(&self.triggerC, &default_trigger_button_x1());

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
        migrate_legacy_wheel_actions(&mut self.actions);
        self.groups = normalize_groups_and_actions(self.groups, &mut self.actions);

        self
    }

    fn validate(&self) -> Result<(), ValidationError> {
        if !is_valid_trigger_binding(&self.triggerA) {
            return Err(ValidationError::InvalidValue(format!(
                "invalid triggerA: {}",
                self.triggerA
            )));
        }
        if !is_valid_trigger_binding(&self.triggerB) {
            return Err(ValidationError::InvalidValue(format!(
                "invalid triggerB: {}",
                self.triggerB
            )));
        }
        if !is_valid_trigger_binding(&self.triggerC) {
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

    fn left_click_trigger_slots(&self) -> Vec<&'static str> {
        let mut slots = Vec::new();
        if is_left_mouse_trigger(&self.triggerA) {
            slots.push("A");
        }
        if is_left_mouse_trigger(&self.triggerB) {
            slots.push("B");
        }
        if is_left_mouse_trigger(&self.triggerC) {
            slots.push("C");
        }
        slots
    }
}

fn sanitize_left_click_triggers_in_raw_json(content: &str) -> (String, Vec<&'static str>) {
    fn sanitize_slot(
        content: &str,
        key: &str,
        slot: &'static str,
    ) -> Option<(std::ops::Range<usize>, &'static str)> {
        let key_pos = content.find(key)?;
        let bytes = content.as_bytes();
        let mut cursor = key_pos + key.len();

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b':' {
            return None;
        }
        cursor += 1;

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'"' {
            return None;
        }

        let value_start = cursor + 1;
        let value_end = content[value_start..].find('"').map(|offset| value_start + offset)?;
        if is_left_mouse_trigger(&content[value_start..value_end]) {
            Some((value_start..value_end, slot))
        } else {
            None
        }
    }

    let mut content = content.to_string();
    let mut slots = Vec::new();
    for (key, slot) in [
        ("\"triggerA\"", "A"),
        ("\"triggerB\"", "B"),
        ("\"triggerC\"", "C"),
    ] {
        if let Some((range, slot_name)) = sanitize_slot(&content, key, slot) {
            content.replace_range(range, UNASSIGNED_TRIGGER);
            slots.push(slot_name);
        }
    }

    (content, slots)
}

/// 旧仕様の "leftclick_wheel_up"/"leftclick_wheel_down" は左クリック押下中の
/// ホイール操作という廃止済みモデル向けの値。新モデルでは Trigger A/B/C +
/// ホイール方向のみをサポートするため、対応する通常方向へ移行する。
/// 移行先スロットは既存の trigger_slot（空なら "A"）を優先し、既に同じ
/// スロット+方向の組み合わせが使用中であれば A→B→C の順で空きスロットを
/// 探す。アクション自体（名前・実行内容）は一切変更せず、削除もしない。
fn migrate_legacy_wheel_actions(actions: &mut [Action]) {
    let mut occupied: HashSet<(String, String)> = HashSet::new();
    for action in actions.iter() {
        if action.trigger_type == "wheel" {
            if let Some(direction) = action.wheel_trigger.as_deref() {
                if direction == "wheel_up" || direction == "wheel_down" {
                    occupied.insert((action.trigger_slot.clone(), direction.to_string()));
                }
            }
        }
    }

    for action in actions.iter_mut() {
        if action.trigger_type != "wheel" {
            continue;
        }

        let direction = match action.wheel_trigger.as_deref() {
            Some("leftclick_wheel_up") => "wheel_up",
            Some("leftclick_wheel_down") => "wheel_down",
            _ => continue,
        };

        let mut chosen_slot = if action.trigger_slot.is_empty() {
            default_trigger_slot()
        } else {
            action.trigger_slot.clone()
        };

        if occupied.contains(&(chosen_slot.clone(), direction.to_string())) {
            if let Some(free_slot) = ["A", "B", "C"]
                .iter()
                .map(|s| s.to_string())
                .find(|s| !occupied.contains(&(s.clone(), direction.to_string())))
            {
                chosen_slot = free_slot;
            }
            // 全スロットが埋まっている場合でも、アクションは失わずに元の
            // スロットへ移行する（ランタイム側は先勝ちで解決する）。
        }

        occupied.insert((chosen_slot.clone(), direction.to_string()));
        action.trigger_slot = chosen_slot;
        action.wheel_trigger = Some(direction.to_string());
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
        let (content, raw_left_click_slots) = sanitize_left_click_triggers_in_raw_json(&content);
        if !raw_left_click_slots.is_empty() {
            let backup_dir = self.backup_settings_files()?;
            fs::write(&path, &content)
                .map_err(|e| format!("Failed to rewrite sanitized config.json: {}", e))?;
            eprintln!(
                "[config] sanitized left-click trigger(s) in raw config.json: slots={} backup={}",
                raw_left_click_slots.join(","),
                backup_dir.display()
            );
        }

        let parsed: Config = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config.json: {}", e))?;
        let left_click_slots = parsed.left_click_trigger_slots();
        let normalized = parsed.clone().normalized();

        if let Err(e) = normalized.validate() {
            return Err(format!("config.json validation error: {}", e));
        }

        if normalized != parsed {
            if !left_click_slots.is_empty() {
                let backup_dir = self.backup_settings_files()?;
                eprintln!(
                    "[config] sanitized left-click trigger(s) in config.json: slots={} backup={}",
                    left_click_slots.join(","),
                    backup_dir.display()
                );
            }
            self.save_config(&normalized)?;
        }

        Ok(normalized)
    }

    pub fn save_config(&self, config: &Config) -> Result<(), String> {
        let left_click_slots = config.left_click_trigger_slots();
        if !left_click_slots.is_empty() {
            return Err(format!(
                "Left click cannot be used as a trigger. Invalid slots: {}",
                left_click_slots.join(", ")
            ));
        }

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

    /// 破壊的な書き込み（デフォルトへのリセット等）の直前に必ず呼び出し、
    /// 既存の config.json / gestures.json を退避する。呼び出し漏れがあると
    /// ユーザーのカスタムアクションが復元不能なまま失われるため公開する。
    pub fn backup_before_destructive_write(&self) -> Result<PathBuf, String> {
        self.backup_settings_files()
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

        let sanitized_slots = bundle.config.left_click_trigger_slots();
        let normalized_config = bundle.config.normalized();
        if let Err(e) = normalized_config.validate() {
            return Err(format!("settings bundle config validation error: {}", e));
        }

        if !sanitized_slots.is_empty() {
            eprintln!(
                "[config] sanitized left-click trigger(s) from imported settings bundle: slots={}",
                sanitized_slots.join(",")
            );
        }

        self.save_gestures(&bundle.gestures)?;
        self.save_config(&normalized_config)?;
        Ok(())
    }

    fn backup_settings_files(&self) -> Result<PathBuf, String> {
        let timestamp = chrono_like_backup_timestamp();
        let mut backup_dir = self.config_dir.join(format!("backup-{}", timestamp));
        let mut suffix = 1usize;
        while backup_dir.exists() {
            backup_dir = self
                .config_dir
                .join(format!("backup-{}-{}", timestamp, suffix));
            suffix += 1;
        }

        fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;

        for file_name in ["config.json", "gestures.json"] {
            let source = self.config_dir.join(file_name);
            if source.exists() {
                let target = backup_dir.join(file_name);
                fs::copy(&source, &target).map_err(|e| {
                    format!(
                        "Failed to back up {} to {}: {}",
                        source.display(),
                        target.display(),
                        e
                    )
                })?;
            }
        }

        Ok(backup_dir)
    }
}

fn chrono_like_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}", duration.as_secs()),
        Err(_) => "0".to_string(),
    }
}

fn chrono_like_backup_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Clone, Copy)]
    struct DateTimeParts {
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    }

    fn civil_from_days(days: i64) -> (i32, u32, u32) {
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = mp + if mp < 10 { 3 } else { -9 };
        let year = y + if month <= 2 { 1 } else { 0 };
        (year as i32, month as u32, day as u32)
    }

    fn utc_parts_from_unix_seconds(unix_seconds: i64) -> DateTimeParts {
        let days = unix_seconds.div_euclid(86_400);
        let seconds_of_day = unix_seconds.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        let hour = (seconds_of_day / 3_600) as u32;
        let minute = ((seconds_of_day % 3_600) / 60) as u32;
        let second = (seconds_of_day % 60) as u32;
        DateTimeParts {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    let unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let dt = utc_parts_from_unix_seconds(unix_seconds);
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("openmousegesture-{}-{}", label, unique));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    #[test]
    fn legacy_mouse_triggers_migrate_to_unified_format() {
        assert_eq!(normalize_trigger_binding("right", "mouse:right"), "mouse:right");
        assert_eq!(normalize_trigger_binding("middle", "mouse:middle"), "mouse:middle");
        assert_eq!(normalize_trigger_binding("x1", "mouse:x1"), "mouse:x1");
        assert_eq!(normalize_trigger_binding("x2", "mouse:x2"), "mouse:x2");
    }

    #[test]
    fn unified_mouse_triggers_round_trip() {
        for value in ["mouse:right", "mouse:middle", "mouse:x1", "mouse:x2"] {
            assert_eq!(normalize_trigger_binding(value, "mouse:right"), value);
        }
    }

    #[test]
    fn left_click_is_never_a_valid_trigger_binding() {
        // Left click must never be registerable: it must be cleared to the
        // unassigned state instead of being normalized to "mouse:left", and
        // must fail raw validation if it somehow appears on disk.
        for input in ["left", "mouse:left", "Left", "MOUSE:LEFT", " mouse:left "] {
            assert_eq!(normalize_trigger_binding(input, "mouse:right"), UNASSIGNED_TRIGGER);
            assert!(!is_valid_trigger_binding(input));
        }
    }

    #[test]
    fn keyboard_triggers_parse_with_ordered_modifiers() {
        let (modifiers, code) = parse_keyboard_trigger("key:Shift+F1").expect("should parse");
        assert_eq!(modifiers, vec!["Shift".to_string()]);
        assert_eq!(code, "F1");

        let (modifiers, code) = parse_keyboard_trigger("key:Alt+Ctrl+KeyK").expect("should parse");
        assert_eq!(modifiers, vec!["Ctrl".to_string(), "Alt".to_string()]);
        assert_eq!(code, "KeyK");
    }

    #[test]
    fn keyboard_trigger_formatting_is_stable_regardless_of_input_order() {
        let normalized = normalize_trigger_binding("key:Shift+Alt+KeyK", "mouse:right");
        assert_eq!(normalized, "key:Alt+Shift+KeyK");
    }

    #[test]
    fn modifier_only_keyboard_trigger_is_rejected() {
        assert!(parse_keyboard_trigger("key:Shift").is_none());
        assert!(parse_keyboard_trigger("key:Ctrl+Alt").is_none());
    }

    #[test]
    fn unknown_key_code_is_rejected() {
        assert!(parse_keyboard_trigger("key:NotARealKey").is_none());
    }

    #[test]
    fn invalid_trigger_binding_falls_back_to_default() {
        let fallback = normalize_trigger_binding("garbage", "mouse:right");
        assert_eq!(fallback, "mouse:right");
    }

    #[test]
    fn config_normalization_migrates_all_legacy_slots_and_is_idempotent() {
        let mut config = Config::default();
        config.triggerA = "right".to_string();
        config.triggerB = "middle".to_string();
        config.triggerC = "x1".to_string();

        let normalized = config.normalized();
        assert_eq!(normalized.triggerA, "mouse:right");
        assert_eq!(normalized.triggerB, "mouse:middle");
        assert_eq!(normalized.triggerC, "mouse:x1");
        assert!(normalized.validate().is_ok());

        let twice_normalized = normalized.clone().normalized();
        assert_eq!(normalized, twice_normalized);
    }

    #[test]
    fn config_normalization_sanitizes_left_click_trigger_to_unassigned() {
        // Mirrors the real-world lockout: a config.json (hand-edited or imported)
        // with triggerA set to left click must never activate; normalization
        // must clear the dangerous slot without changing the other triggers.
        let mut config = Config::default();
        config.triggerA = "mouse:left".to_string();
        config.triggerB = "left".to_string();

        let normalized = config.normalized();
        assert_eq!(normalized.triggerA, UNASSIGNED_TRIGGER);
        assert_eq!(normalized.triggerB, UNASSIGNED_TRIGGER);
        assert!(normalized.validate().is_ok());
    }

    #[test]
    fn unassigned_trigger_binding_is_valid_and_stable() {
        assert_eq!(normalize_trigger_binding("", "mouse:right"), UNASSIGNED_TRIGGER);
        assert!(is_valid_trigger_binding(""));
        assert!(Config {
            triggerA: UNASSIGNED_TRIGGER.to_string(),
            ..Config::default()
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn save_config_rejects_left_click_trigger_values() {
        let temp_dir = unique_temp_dir("save-rejects-left");
        let manager = ConfigManager {
            config_dir: temp_dir.clone(),
        };
        let mut config = Config::default();
        config.triggerA = "mouse:left".to_string();

        let error = manager
            .save_config(&config)
            .expect_err("left click should be rejected before saving");
        assert!(error.contains("Left click cannot be used as a trigger."));
        assert!(!temp_dir.join("config.json").exists());
    }

    #[test]
    fn load_config_sanitizes_left_click_and_creates_backup() {
        let temp_dir = unique_temp_dir("load-sanitizes-left");
        let manager = ConfigManager {
            config_dir: temp_dir.clone(),
        };
        fs::write(
            temp_dir.join("gestures.json"),
            include_str!("../../config/default-gestures.json"),
        )
        .expect("gestures should be written");
        fs::write(
            temp_dir.join("config.json"),
            serde_json::to_string_pretty(&Config {
                triggerA: "mouse:left".to_string(),
                triggerB: "mouse:middle".to_string(),
                triggerC: "mouse:x1".to_string(),
                ..Config::default()
            })
            .expect("config should serialize"),
        )
        .expect("config should be written");

        let loaded = manager.load_config().expect("config should load");
        assert_eq!(loaded.triggerA, UNASSIGNED_TRIGGER);
        assert_eq!(loaded.triggerB, "mouse:middle");
        assert_eq!(loaded.triggerC, "mouse:x1");

        let persisted: Config = serde_json::from_str(
            &fs::read_to_string(temp_dir.join("config.json")).expect("sanitized config should exist"),
        )
        .expect("sanitized config should parse");
        assert_eq!(persisted.triggerA, UNASSIGNED_TRIGGER);

        let backup_dir = fs::read_dir(&temp_dir)
            .expect("backup dir listing should succeed")
            .filter_map(Result::ok)
            .find(|entry| entry.file_name().to_string_lossy().starts_with("backup-"))
            .map(|entry| entry.path())
            .expect("backup directory should exist");
        let backup_config: Config = serde_json::from_str(
            &fs::read_to_string(backup_dir.join("config.json")).expect("backup config should exist"),
        )
        .expect("backup config should parse");
        assert_eq!(backup_config.triggerA, "mouse:left");
    }

    #[test]
    fn raw_json_left_click_sanitization_rewrites_only_trigger_values() {
        let raw = r#"{
  "triggerA" : "mouse:left",
  "triggerB":"left",
  "triggerC": "mouse:x1",
  "notes":"keep-left-as-text"
}"#;

        let (sanitized, slots) = sanitize_left_click_triggers_in_raw_json(raw);
        assert_eq!(slots, vec!["A", "B"]);
        assert!(sanitized.contains(r#""triggerA" : """#));
        assert!(sanitized.contains(r#""triggerB":"""#));
        assert!(sanitized.contains(r#""triggerC": "mouse:x1""#));
        assert!(sanitized.contains(r#""notes":"keep-left-as-text""#));
    }

    #[test]
    fn imported_settings_bundle_sanitizes_left_click_trigger() {
        let temp_dir = unique_temp_dir("import-sanitizes-left");
        let manager = ConfigManager {
            config_dir: temp_dir.clone(),
        };
        let bundle = SettingsBundle {
            formatVersion: 1,
            appName: "GestureHotkeyApp".to_string(),
            exportedAt: "0".to_string(),
            config: Config {
                triggerA: "mouse:left".to_string(),
                triggerB: "key:Shift+F1".to_string(),
                triggerC: "mouse:x2".to_string(),
                ..Config::default()
            },
            gestures: serde_json::from_str(include_str!("../../config/default-gestures.json"))
                .expect("default gestures should parse"),
        };

        manager
            .import_settings_bundle(bundle)
            .expect("import should sanitize left click");
        let imported = manager.load_config().expect("imported config should load");
        assert_eq!(imported.triggerA, UNASSIGNED_TRIGGER);
        assert_eq!(imported.triggerB, "key:Shift+F1");
        assert_eq!(imported.triggerC, "mouse:x2");
    }

    #[test]
    fn keyboard_code_to_vk_covers_function_and_letter_keys() {
        assert_eq!(keyboard_code_to_vk("F1"), Some(0x70));
        assert_eq!(keyboard_code_to_vk("KeyK"), Some(b'K' as u16));
        assert_eq!(keyboard_code_to_vk("Digit5"), Some(b'5' as u16));
        assert_eq!(keyboard_code_to_vk("NotAKey"), None);
    }

    fn sample_rich_config(action_count: usize) -> Config {
        let mut config = Config {
            triggerA: "mouse:middle".to_string(),
            triggerB: "mouse:right".to_string(),
            triggerC: "mouse:x1".to_string(),
            groups: vec![
                ActionGroup { id: "group-general".to_string(), name: "一般".to_string() },
                ActionGroup { id: "group-clipboard".to_string(), name: "クリップボード".to_string() },
            ],
            ..Config::default()
        };

        for i in 0..action_count {
            config.actions.push(Action {
                name: format!("action-{}", i),
                group_id: "group-general".to_string(),
                trigger_type: "gesture".to_string(),
                trigger_slot: ["A", "B", "C"][i % 3].to_string(),
                gesture: format!("gesture-{}", i),
                action_type: "keystroke".to_string(),
                keystroke: Some("K".to_string()),
                modifiers: Some(vec!["Ctrl".to_string()]),
                ..Action::default()
            });
        }

        config
    }

    #[test]
    fn valid_custom_20_action_config_survives_load_save_unchanged() {
        let temp_dir = unique_temp_dir("custom-20-actions-roundtrip");
        let manager = ConfigManager { config_dir: temp_dir.clone() };
        let original = sample_rich_config(20);

        manager.save_config(&original).expect("custom config should save");
        let loaded = manager.load_config().expect("custom config should load");

        assert_eq!(loaded.actions.len(), 20);
        assert_eq!(
            loaded.actions.iter().map(|a| a.name.clone()).collect::<Vec<_>>(),
            original.actions.iter().map(|a| a.name.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn missing_optional_fields_are_filled_without_dropping_actions() {
        let temp_dir = unique_temp_dir("missing-fields-fill");
        let manager = ConfigManager { config_dir: temp_dir.clone() };

        // Hand-write JSON missing trigger_slot/group_id-normalization inputs,
        // mirroring an older config version with fewer fields per action.
        let raw = r#"{
  "trajectory": true,
  "ignore_exe": [],
  "triggerA": "mouse:middle",
  "triggerB": "mouse:right",
  "triggerC": "mouse:x1",
  "groups": [{"id": "group-general", "name": "一般"}],
  "actions": [
    {"name": "a1", "group_id": "group-general", "gesture": "左", "action_type": "keystroke", "keystroke": "A"},
    {"name": "a2", "group_id": "group-general", "gesture": "右", "action_type": "keystroke", "keystroke": "B"}
  ]
}"#;
        fs::write(temp_dir.join("config.json"), raw).expect("raw config should write");

        let loaded = manager.load_config().expect("config with missing optional fields should load");
        assert_eq!(loaded.actions.len(), 2);
        // defaults fill in trigger_type/trigger_slot but do not touch the actions themselves
        assert_eq!(loaded.actions[0].trigger_type, "gesture");
        assert!(is_valid_trigger_slot(&loaded.actions[0].trigger_slot));
    }

    #[test]
    fn malformed_config_is_preserved_not_silently_replaced() {
        let temp_dir = unique_temp_dir("malformed-preserved");
        let manager = ConfigManager { config_dir: temp_dir.clone() };
        let malformed = "{ this is not valid json";
        fs::write(temp_dir.join("config.json"), malformed).expect("malformed config should write");

        let result = manager.load_config();
        assert!(result.is_err(), "malformed config must surface an error, not silently succeed");

        // The original malformed file must remain untouched on disk - no
        // silent overwrite with defaults happened.
        let on_disk = fs::read_to_string(temp_dir.join("config.json")).expect("file should still exist");
        assert_eq!(on_disk, malformed);
    }

    #[test]
    fn first_run_creates_default_config() {
        let temp_dir = unique_temp_dir("first-run-defaults");
        let manager = ConfigManager { config_dir: temp_dir.clone() };
        assert!(!temp_dir.join("config.json").exists());

        let loaded = manager.load_config().expect("first run should create defaults");
        assert!(!loaded.actions.is_empty());
        assert!(temp_dir.join("config.json").exists());
    }

    #[test]
    fn repeated_startup_load_is_idempotent_and_preserves_custom_actions() {
        let temp_dir = unique_temp_dir("idempotent-startup");
        let manager = ConfigManager { config_dir: temp_dir.clone() };
        let original = sample_rich_config(21);
        manager.save_config(&original).expect("custom config should save");

        let first_load = manager.load_config().expect("first load should succeed");
        let second_load = manager.load_config().expect("second load should succeed");
        let third_load = manager.load_config().expect("third load should succeed");

        assert_eq!(first_load.actions.len(), 21);
        assert_eq!(second_load.actions.len(), 21);
        assert_eq!(third_load.actions.len(), 21);
        assert_eq!(first_load, second_load);
        assert_eq!(second_load, third_load);
    }

    #[test]
    fn five_action_defaults_never_replace_a_valid_richer_custom_set() {
        let temp_dir = unique_temp_dir("no-default-override");
        let manager = ConfigManager { config_dir: temp_dir.clone() };
        let rich = sample_rich_config(21);
        manager.save_config(&rich).expect("rich config should save");

        // Loading multiple times must never collapse the custom action set
        // down to the bundled 5-action default template.
        for _ in 0..3 {
            let loaded = manager.load_config().expect("load should succeed");
            assert_eq!(loaded.actions.len(), 21, "custom actions must not be replaced by defaults");
        }
    }

    #[test]
    fn left_click_sanitation_preserves_actions_array() {
        let temp_dir = unique_temp_dir("left-click-preserves-actions");
        let manager = ConfigManager { config_dir: temp_dir.clone() };
        let mut config = sample_rich_config(21);
        config.triggerA = "mouse:left".to_string();

        // save_config rejects left-click directly, so write the file with a
        // left-click trigger the way a hand-edited or legacy file would.
        let raw = serde_json::to_string_pretty(&config).expect("config should serialize");
        fs::write(temp_dir.join("config.json"), raw).expect("config should write");

        let loaded = manager.load_config().expect("config should load and sanitize");
        assert_eq!(loaded.triggerA, UNASSIGNED_TRIGGER);
        assert_eq!(loaded.actions.len(), 21, "sanitizing left-click must not drop actions");
    }

    #[test]
    fn destructive_reset_backs_up_existing_files_first() {
        let temp_dir = unique_temp_dir("destructive-reset-backup");
        let manager = ConfigManager { config_dir: temp_dir.clone() };
        let rich = sample_rich_config(21);
        manager.save_config(&rich).expect("rich config should save");
        manager
            .save_gestures(&[GestureTemplate { name: "左".to_string(), points: vec![(0.0, 0.0), (1.0, 1.0)] }])
            .expect("gestures should save");

        let backup_dir = manager
            .backup_before_destructive_write()
            .expect("backup should succeed before any destructive fallback write");

        let backed_up_config: Config = serde_json::from_str(
            &fs::read_to_string(backup_dir.join("config.json")).expect("backup config should exist"),
        )
        .expect("backup config should parse");
        assert_eq!(backed_up_config.actions.len(), 21, "backup must capture the full custom set");
        assert!(backup_dir.join("gestures.json").exists());
    }

    #[test]
    fn serialization_reload_preserves_restored_actions() {
        let temp_dir = unique_temp_dir("serialization-reload");
        let manager = ConfigManager { config_dir: temp_dir.clone() };
        let restored = sample_rich_config(21);

        manager.save_config(&restored).expect("restored config should save");
        let reloaded = manager.load_config().expect("restored config should reload");

        assert_eq!(reloaded.actions.len(), restored.actions.len());
        for (original, reloaded_action) in restored.actions.iter().zip(reloaded.actions.iter()) {
            assert_eq!(original.name, reloaded_action.name);
            assert_eq!(original.gesture, reloaded_action.gesture);
            assert_eq!(original.action_type, reloaded_action.action_type);
            assert_eq!(original.keystroke, reloaded_action.keystroke);
        }
    }

    #[test]
    fn editing_action_group_id_moves_it_to_the_destination_group() {
        let mut config = sample_rich_config(3);
        assert!(config.actions.iter().all(|a| a.group_id == "group-general"));

        config.actions[1].group_id = "group-clipboard".to_string();
        let normalized = config.normalized();

        let moved = normalized
            .actions
            .iter()
            .find(|a| a.name == "action-1")
            .expect("moved action must still exist");
        assert_eq!(moved.group_id, "group-clipboard");

        // Untouched actions keep their original group assignment.
        assert_eq!(normalized.actions[0].group_id, "group-general");
        assert_eq!(normalized.actions[2].group_id, "group-general");

        // The action appears exactly once across the whole action list -
        // moving groups must never duplicate it.
        assert_eq!(
            normalized.actions.iter().filter(|a| a.name == "action-1").count(),
            1
        );
        assert_eq!(normalized.actions.len(), 3, "no action must be dropped or duplicated by a group move");
    }

    #[test]
    fn moved_action_preserves_identity_and_all_other_fields() {
        let mut config = sample_rich_config(3);
        let original = config.actions[1].clone();

        config.actions[1].group_id = "group-clipboard".to_string();
        let normalized = config.normalized();
        let moved = normalized.actions.iter().find(|a| a.name == "action-1").unwrap();

        assert_eq!(moved.name, original.name);
        assert_eq!(moved.trigger_type, original.trigger_type);
        assert_eq!(moved.trigger_slot, original.trigger_slot);
        assert_eq!(moved.gesture, original.gesture);
        assert_eq!(moved.action_type, original.action_type);
        assert_eq!(moved.keystroke, original.keystroke);
        assert_eq!(moved.modifiers, original.modifiers);
        assert_ne!(moved.group_id, original.group_id, "only group_id should have changed");
    }

    #[test]
    fn moving_to_each_existing_group_resolves_to_that_group() {
        let config = sample_rich_config(1);
        let groups = config.groups.clone();

        for group in &groups {
            let mut candidate = config.clone();
            candidate.actions[0].group_id = group.id.clone();
            let normalized = candidate.normalized();
            assert_eq!(normalized.actions[0].group_id, group.id);
            // Moving must never invent a duplicate of an already-known group.
            assert_eq!(
                normalized.groups.iter().filter(|g| g.id == group.id).count(),
                1
            );
        }
    }

    #[test]
    fn missing_or_deleted_group_id_falls_back_to_default_group_safely() {
        let mut config = sample_rich_config(2);
        config.actions[0].group_id = "group-does-not-exist".to_string();

        let normalized = config.normalized();

        assert_eq!(normalized.actions[0].group_id, DEFAULT_GROUP_ID, "an action referencing a deleted group must fall back to the default group, not be dropped");
        assert_eq!(normalized.actions.len(), 2, "the action must survive the fallback, not be dropped");
        assert!(normalized.groups.iter().any(|g| g.id == DEFAULT_GROUP_ID));
    }

    #[test]
    fn group_reassignment_survives_save_and_reload() {
        let temp_dir = unique_temp_dir("group-reassignment-roundtrip");
        let manager = ConfigManager { config_dir: temp_dir.clone() };
        let mut config = sample_rich_config(5);
        manager.save_config(&config).expect("initial config should save");

        config.actions[2].group_id = "group-clipboard".to_string();
        manager.save_config(&config).expect("reassigned config should save");

        let reloaded = manager.load_config().expect("reassigned config should reload");
        assert_eq!(reloaded.actions.len(), 5, "reload must not drop or duplicate actions");
        assert_eq!(reloaded.actions[2].group_id, "group-clipboard");
        assert_eq!(
            reloaded.actions.iter().filter(|a| a.name == "action-2").count(),
            1,
            "the reassigned action must appear exactly once after reload"
        );
        for (idx, action) in reloaded.actions.iter().enumerate() {
            if idx != 2 {
                assert_eq!(action.group_id, "group-general", "unrelated actions must keep their original group");
            }
        }
    }

    fn wheel_action(trigger_slot: &str, wheel_trigger: &str) -> Action {
        Action {
            trigger_type: "wheel".to_string(),
            trigger_slot: trigger_slot.to_string(),
            wheel_trigger: Some(wheel_trigger.to_string()),
            action_type: "keystroke".to_string(),
            keystroke: Some("PageDown".to_string()),
            group_id: DEFAULT_GROUP_ID.to_string(),
            ..Action::default()
        }
    }

    #[test]
    fn wheel_action_gets_default_slot_a_when_missing() {
        let action = wheel_action("", "wheel_up").normalized();
        assert_eq!(action.trigger_slot, "A");
        assert_eq!(action.wheel_trigger.as_deref(), Some("wheel_up"));
    }

    #[test]
    fn wheel_action_slot_is_uppercased() {
        let action = wheel_action("b", "wheel_down").normalized();
        assert_eq!(action.trigger_slot, "B");
    }

    #[test]
    fn migrate_legacy_wheel_actions_converts_leftclick_variants_retaining_slot() {
        let mut actions = vec![
            wheel_action("B", "leftclick_wheel_up").normalized(),
            wheel_action("C", "leftclick_wheel_down").normalized(),
        ];
        migrate_legacy_wheel_actions(&mut actions);

        assert_eq!(actions[0].trigger_slot, "B");
        assert_eq!(actions[0].wheel_trigger.as_deref(), Some("wheel_up"));
        assert_eq!(actions[1].trigger_slot, "C");
        assert_eq!(actions[1].wheel_trigger.as_deref(), Some("wheel_down"));
    }

    #[test]
    fn migrate_legacy_wheel_actions_defaults_missing_slot_to_a() {
        let mut actions = vec![wheel_action("", "leftclick_wheel_up")];
        migrate_legacy_wheel_actions(&mut actions);
        assert_eq!(actions[0].trigger_slot, "A");
        assert_eq!(actions[0].wheel_trigger.as_deref(), Some("wheel_up"));
    }

    #[test]
    fn migrate_legacy_wheel_actions_avoids_colliding_with_existing_wheel_up_action() {
        // Slot A already has a real "wheel_up" action; a legacy leftclick_wheel_up
        // action defaulting to slot A must move to a free slot instead of colliding
        // and must never be dropped.
        let mut actions = vec![
            wheel_action("A", "wheel_up"),
            wheel_action("", "leftclick_wheel_up"),
        ];
        migrate_legacy_wheel_actions(&mut actions);

        assert_eq!(actions.len(), 2, "no action may be dropped during migration");
        assert_eq!(actions[0].trigger_slot, "A");
        assert_eq!(actions[0].wheel_trigger.as_deref(), Some("wheel_up"));
        assert_eq!(actions[1].wheel_trigger.as_deref(), Some("wheel_up"));
        assert_ne!(
            actions[1].trigger_slot, "A",
            "legacy action must be reassigned to a free slot rather than colliding"
        );
    }

    #[test]
    fn migrate_legacy_wheel_actions_keeps_action_when_all_slots_occupied() {
        let mut actions = vec![
            wheel_action("A", "wheel_up"),
            wheel_action("B", "wheel_up"),
            wheel_action("C", "wheel_up"),
            wheel_action("", "leftclick_wheel_up"),
        ];
        migrate_legacy_wheel_actions(&mut actions);

        assert_eq!(actions.len(), 4, "action must be retained even when every slot is occupied");
        assert_eq!(actions[3].wheel_trigger.as_deref(), Some("wheel_up"));
    }

    #[test]
    fn migrate_legacy_wheel_actions_leaves_modern_actions_untouched() {
        let mut actions = vec![wheel_action("B", "wheel_down")];
        let before = actions.clone();
        migrate_legacy_wheel_actions(&mut actions);
        assert_eq!(actions, before);
    }

    #[test]
    fn config_normalized_migrates_legacy_wheel_actions_end_to_end() {
        let mut config = Config::default();
        config.actions = vec![wheel_action("", "leftclick_wheel_down")];
        let normalized = config.normalized();

        assert_eq!(normalized.actions.len(), 1, "legacy wheel action must survive normalization");
        assert_eq!(normalized.actions[0].wheel_trigger.as_deref(), Some("wheel_down"));
        assert_eq!(normalized.actions[0].trigger_slot, "A");
    }

    #[test]
    fn wheel_action_validate_rejects_invalid_trigger_slot() {
        let mut action = wheel_action("A", "wheel_up").normalized();
        action.trigger_slot = "Z".to_string();
        let known_groups: HashSet<String> = [DEFAULT_GROUP_ID.to_string()].into_iter().collect();
        assert!(action.validate(&known_groups).is_err());
    }

    #[test]
    fn wheel_action_validate_accepts_wheel_up_and_down_per_slot() {
        let known_groups: HashSet<String> = [DEFAULT_GROUP_ID.to_string()].into_iter().collect();
        for slot in ["A", "B", "C"] {
            for direction in ["wheel_up", "wheel_down"] {
                let action = wheel_action(slot, direction).normalized();
                assert!(action.validate(&known_groups).is_ok());
            }
        }
    }

    // --- `text` action type: literal Unicode text insertion, distinct from `command` ---

    fn text_action(text: &str) -> Action {
        Action {
            trigger_type: "gesture".to_string(),
            trigger_slot: "A".to_string(),
            gesture: "左".to_string(),
            action_type: "text".to_string(),
            text: Some(text.to_string()),
            group_id: DEFAULT_GROUP_ID.to_string(),
            ..Action::default()
        }
    }

    #[test]
    fn text_action_type_is_accepted_by_validate() {
        let known_groups: HashSet<String> = [DEFAULT_GROUP_ID.to_string()].into_iter().collect();
        let action = text_action("hello@example.com").normalized();
        assert!(action.validate(&known_groups).is_ok());
    }

    #[test]
    fn text_action_requires_non_empty_text_field() {
        let known_groups: HashSet<String> = [DEFAULT_GROUP_ID.to_string()].into_iter().collect();

        let missing = text_action("");
        let missing = Action { text: None, ..missing };
        assert!(missing.validate(&known_groups).is_err());

        let blank = text_action("   \n  ").normalized();
        assert!(blank.validate(&known_groups).is_err());
    }

    #[test]
    fn text_action_accepts_japanese_punctuation_and_multiline_content() {
        let known_groups: HashSet<String> = [DEFAULT_GROUP_ID.to_string()].into_iter().collect();
        let action = text_action("こんにちは、世界！\n2行目です。 記号!@#￥%").normalized();
        assert!(action.validate(&known_groups).is_ok());
    }

    #[test]
    fn text_action_does_not_reuse_or_require_command_field() {
        // `text` and `command` must remain distinct: a text action does not need
        // (and should not require) the `command` field to validate successfully.
        let known_groups: HashSet<String> = [DEFAULT_GROUP_ID.to_string()].into_iter().collect();
        let action = text_action("plain text body").normalized();
        assert!(action.command.is_none());
        assert!(action.validate(&known_groups).is_ok());
    }

    #[test]
    fn command_action_type_still_requires_no_text_field() {
        // A `command` action must remain a launcher; it must not require or read
        // the new `text` field, and omitting `text` must not fail validation.
        let action = Action {
            trigger_type: "gesture".to_string(),
            trigger_slot: "A".to_string(),
            gesture: "右".to_string(),
            action_type: "command".to_string(),
            command: Some("notepad.exe".to_string()),
            group_id: DEFAULT_GROUP_ID.to_string(),
            ..Action::default()
        }
        .normalized();
        let known_groups: HashSet<String> = [DEFAULT_GROUP_ID.to_string()].into_iter().collect();
        assert!(action.text.is_none());
        assert!(action.validate(&known_groups).is_ok());
    }

    #[test]
    fn text_action_serialization_round_trips_unicode_and_line_breaks() {
        let temp_dir = unique_temp_dir("text-action-roundtrip");
        let manager = ConfigManager { config_dir: temp_dir.clone() };

        let mut config = Config::default();
        config.actions = vec![text_action("メール: user@example.co.jp\n複数行\nテキスト").normalized()];
        config.groups = vec![ActionGroup { id: DEFAULT_GROUP_ID.to_string(), name: "未分類".to_string() }];

        manager.save_config(&config).expect("config with text action should save");
        let reloaded = manager.load_config().expect("config with text action should reload");

        assert_eq!(reloaded.actions.len(), 1);
        assert_eq!(reloaded.actions[0].action_type, "text");
        assert_eq!(
            reloaded.actions[0].text.as_deref(),
            Some("メール: user@example.co.jp\n複数行\nテキスト")
        );
    }

    #[test]
    fn config_without_text_field_loads_with_backward_compatible_default() {
        // Older configs on disk never had a `text` field at all. Loading one must
        // not fail, and every action's `text` must default to None.
        let temp_dir = unique_temp_dir("legacy-config-no-text-field");
        let manager = ConfigManager { config_dir: temp_dir.clone() };

        let legacy_json = r#"{
            "trajectory": true,
            "ignore_exe": [],
            "triggerA": "mouse:right",
            "triggerB": "mouse:middle",
            "triggerC": "mouse:x1",
            "groups": [{"id": "group-general", "name": "一般"}],
            "actions": [
                {"name": "a1", "group_id": "group-general", "gesture": "左", "action_type": "keystroke", "keystroke": "A"}
            ]
        }"#;
        fs::write(temp_dir.join("config.json"), legacy_json).unwrap();

        let loaded = manager.load_config().expect("legacy config without text field should load");
        assert_eq!(loaded.actions.len(), 1);
        assert_eq!(loaded.actions[0].text, None);
    }
}
