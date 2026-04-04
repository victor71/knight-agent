//! Tool Registry
//!
//! Manages tool registration and discovery.

use crate::types::{McpToolDefinition, ToolDefinition, ToolInfo};
use std::collections::HashMap;
use tracing::debug;

/// Registry for managing tools
#[derive(Debug, Clone, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
    mcp_tools: HashMap<String, Vec<McpToolDefinition>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            mcp_tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: ToolDefinition) -> Option<ToolDefinition> {
        debug!("Registering tool: {}", tool.name);
        self.tools.insert(tool.name.clone(), tool)
    }

    /// Unregister a tool
    pub fn unregister(&mut self, name: &str) -> Option<ToolDefinition> {
        debug!("Unregistering tool: {}", name);
        self.tools.remove(name)
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    /// List all registered tools
    pub fn list(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    /// List tools by category
    pub fn list_by_category(&self, category: &str) -> Vec<&ToolDefinition> {
        self.tools
            .values()
            .filter(|t| t.category == category)
            .collect()
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<String> {
        let mut categories: Vec<String> = self.tools
            .values()
            .filter_map(|t| {
                if t.category.is_empty() {
                    None
                } else {
                    Some(t.category.clone())
                }
            })
            .collect();
        categories.sort();
        categories.dedup();
        categories
    }

    /// Convert tool to ToolInfo (without handler)
    pub fn to_info(&self, name: &str) -> Option<ToolInfo> {
        self.tools.get(name).map(|tool| ToolInfo {
            name: tool.name.clone(),
            display_name: tool.display_name.clone(),
            description: tool.description.clone(),
            category: tool.category.clone(),
            parameters: tool.parameters.clone(),
            dangerous: tool.dangerous,
            is_read_only: tool.is_read_only,
        })
    }

    /// List all tools as ToolInfo
    pub fn list_info(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .map(|tool| ToolInfo {
                name: tool.name.clone(),
                display_name: tool.display_name.clone(),
                description: tool.description.clone(),
                category: tool.category.clone(),
                parameters: tool.parameters.clone(),
                dangerous: tool.dangerous,
                is_read_only: tool.is_read_only,
            })
            .collect()
    }

    /// Register MCP tools for a server
    pub fn register_mcp_tools(&mut self, server_name: &str, tools: Vec<McpToolDefinition>) -> usize {
        debug!("Registering {} MCP tools from server: {}", tools.len(), server_name);
        let count = tools.len();
        self.mcp_tools.insert(server_name.to_string(), tools);
        count
    }

    /// Unregister all MCP tools from a server
    pub fn unregister_mcp_server(&mut self, server_name: &str) -> Option<Vec<McpToolDefinition>> {
        debug!("Unregistering MCP server: {}", server_name);
        self.mcp_tools.remove(server_name)
    }

    /// Get MCP tools for a server
    pub fn get_mcp_tools(&self, server_name: &str) -> Option<&Vec<McpToolDefinition>> {
        self.mcp_tools.get(server_name)
    }

    /// List all MCP servers
    pub fn list_mcp_servers(&self) -> Vec<&String> {
        self.mcp_tools.keys().collect()
    }

    /// Check if a tool exists
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get the number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HandlerType, ToolHandler};

    fn create_test_tool(name: &str, category: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            display_name: name.to_string(),
            description: format!("Test tool: {}", name),
            category: category.to_string(),
            parameters: Default::default(),
            handler: ToolHandler {
                handler_type: HandlerType::Builtin,
                target: String::new(),
                timeout_secs: 30,
            },
            permissions: vec![],
            dangerous: false,
            is_read_only: false,
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = ToolRegistry::new();
        let tool = create_test_tool("test_tool", "test");

        registry.register(tool.clone());
        assert!(registry.contains("test_tool"));

        let retrieved = registry.get("test_tool").unwrap();
        assert_eq!(retrieved.name, "test_tool");
    }

    #[test]
    fn test_unregister() {
        let mut registry = ToolRegistry::new();
        registry.register(create_test_tool("test_tool", "test"));

        let removed = registry.unregister("test_tool");
        assert!(removed.is_some());
        assert!(!registry.contains("test_tool"));
    }

    #[test]
    fn test_list_by_category() {
        let mut registry = ToolRegistry::new();
        registry.register(create_test_tool("tool1", "files"));
        registry.register(create_test_tool("tool2", "files"));
        registry.register(create_test_tool("tool3", "search"));

        let files_tools = registry.list_by_category("files");
        assert_eq!(files_tools.len(), 2);

        let search_tools = registry.list_by_category("search");
        assert_eq!(search_tools.len(), 1);
    }

    #[test]
    fn test_categories() {
        let mut registry = ToolRegistry::new();
        registry.register(create_test_tool("tool1", "files"));
        registry.register(create_test_tool("tool2", "search"));
        registry.register(create_test_tool("tool3", "files"));

        let categories = registry.categories();
        assert_eq!(categories, vec!["files", "search"]);
    }

    #[test]
    fn test_mcp_tools() {
        let mut registry = ToolRegistry::new();
        let mcp_tools = vec![
            McpToolDefinition {
                name: "mcp_tool1".to_string(),
                description: "MCP tool 1".to_string(),
                input_schema: serde_json::json!({}),
                server_name: "mcp_server".to_string(),
            },
            McpToolDefinition {
                name: "mcp_tool2".to_string(),
                description: "MCP tool 2".to_string(),
                input_schema: serde_json::json!({}),
                server_name: "mcp_server".to_string(),
            },
        ];

        let count = registry.register_mcp_tools("mcp_server", mcp_tools);
        assert_eq!(count, 2);

        let servers = registry.list_mcp_servers();
        assert_eq!(servers, vec![&"mcp_server".to_string()]);

        let tools = registry.get_mcp_tools("mcp_server").unwrap();
        assert_eq!(tools.len(), 2);
    }
}
