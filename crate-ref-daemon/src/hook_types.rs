use serde::{Deserialize, Serialize};

/// Top-level JSON object Claude Code sends on stdin to PostToolUse hooks.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HookEvent {
    pub hook_event_name: String,
    pub tool_name: String,
    pub tool_input: ToolInput,
    pub tool_response: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ToolInput {
    pub file_path: Option<String>,
    pub command: Option<String>,
    pub content: Option<String>,
}

/// JSON object written to stdout to communicate back to Claude Code.
#[derive(Debug, Serialize, Deserialize)]
pub struct HookResponse {
    /// exit 0 = proceed, exit 2 = block
    pub action: HookAction,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HookAction {
    Proceed,
    Block,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_post_tool_use() {
        let json = r#"{"hook_event_name":"PostToolUse","tool_name":"Write","tool_input":{"file_path":"src/main.rs"},"tool_response":null}"#;
        let event: HookEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.hook_event_name, "PostToolUse");
        assert_eq!(event.tool_name, "Write");
        assert_eq!(event.tool_input.file_path.as_deref(), Some("src/main.rs"));
    }

    #[test]
    fn test_deserialize_missing_file_path() {
        let json = r#"{"hook_event_name":"PostToolUse","tool_name":"Bash","tool_input":{"command":"ls"},"tool_response":null}"#;
        let event: HookEvent = serde_json::from_str(json).unwrap();
        assert!(event.tool_input.file_path.is_none());
    }

    #[test]
    fn test_serialize_proceed_response() {
        let resp = HookResponse {
            action: HookAction::Proceed,
            message: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("proceed"));
    }

    #[test]
    fn test_serialize_block_response() {
        let resp = HookResponse {
            action: HookAction::Block,
            message: Some("error found".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("block"));
        assert!(json.contains("error found"));
    }
}
