use crolens_api::mcp;

#[test]
fn tools_list_has_all_required_tools() {
    let list = mcp::tools::list();
    let tools = list
        .get("tools")
        .and_then(|v| v.as_array())
        .expect("tools must be an array");

    let names = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|v| v.as_str()))
        .collect::<Vec<_>>();

    // All 12 MCP tools
    for required in [
        "get_account_summary",
        "get_defi_positions",
        "decode_transaction",
        "simulate_transaction",
        "search_contract",
        "construct_swap_tx",
        "get_token_info",
        "get_pool_info",
        "get_gas_price",
        "get_token_price",
        "get_approval_status",
        "get_block_info",
    ] {
        assert!(names.contains(&required), "missing tool: {required}");
    }
}

#[test]
fn tools_list_has_correct_count() {
    let list = mcp::tools::list();
    let tools = list
        .get("tools")
        .and_then(|v| v.as_array())
        .expect("tools must be an array");

    assert_eq!(tools.len(), 12, "expected 12 MCP tools");
}

#[test]
fn each_tool_has_required_fields() {
    let list = mcp::tools::list();
    let tools = list
        .get("tools")
        .and_then(|v| v.as_array())
        .expect("tools must be an array");

    for tool in tools {
        let name = tool
            .get("name")
            .and_then(|v| v.as_str())
            .expect("tool must have name");
        assert!(
            tool.get("description").and_then(|v| v.as_str()).is_some(),
            "tool {name} missing description"
        );
        assert!(
            tool.get("inputSchema").is_some(),
            "tool {name} missing inputSchema"
        );
    }
}

#[test]
fn get_approval_status_tool_has_correct_schema() {
    let list = mcp::tools::list();
    let tools = list
        .get("tools")
        .and_then(|v| v.as_array())
        .expect("tools must be an array");

    let tool = tools
        .iter()
        .find(|t| t.get("name").and_then(|v| v.as_str()) == Some("get_approval_status"))
        .expect("get_approval_status tool must exist");

    let schema = tool
        .get("inputSchema")
        .expect("must have inputSchema");

    // Check required fields
    let required = schema
        .get("required")
        .and_then(|v| v.as_array())
        .expect("must have required array");
    assert!(
        required.iter().any(|v| v.as_str() == Some("address")),
        "address must be required"
    );

    // Check properties
    let props = schema
        .get("properties")
        .expect("must have properties");
    assert!(props.get("address").is_some(), "must have address property");
    assert!(props.get("token").is_some(), "must have token property");
    assert!(props.get("simple_mode").is_some(), "must have simple_mode property");
}

#[test]
fn get_block_info_tool_has_correct_schema() {
    let list = mcp::tools::list();
    let tools = list
        .get("tools")
        .and_then(|v| v.as_array())
        .expect("tools must be an array");

    let tool = tools
        .iter()
        .find(|t| t.get("name").and_then(|v| v.as_str()) == Some("get_block_info"))
        .expect("get_block_info tool must exist");

    let schema = tool
        .get("inputSchema")
        .expect("must have inputSchema");

    // Check that no fields are required (block defaults to "latest")
    let required = schema
        .get("required")
        .and_then(|v| v.as_array())
        .expect("must have required array");
    assert!(required.is_empty(), "get_block_info should have no required fields");

    // Check properties
    let props = schema
        .get("properties")
        .expect("must have properties");
    assert!(props.get("block").is_some(), "must have block property");
    assert!(props.get("simple_mode").is_some(), "must have simple_mode property");
}

#[test]
fn all_tools_have_valid_json_schema() {
    let list = mcp::tools::list();
    let tools = list
        .get("tools")
        .and_then(|v| v.as_array())
        .expect("tools must be an array");

    for tool in tools {
        let name = tool
            .get("name")
            .and_then(|v| v.as_str())
            .expect("tool must have name");

        let schema = tool
            .get("inputSchema")
            .expect(&format!("{name} must have inputSchema"));

        // All schemas must have type: object
        let schema_type = schema
            .get("type")
            .and_then(|v| v.as_str())
            .expect(&format!("{name} inputSchema must have type"));
        assert_eq!(schema_type, "object", "{name} inputSchema type must be object");

        // All schemas must have properties
        assert!(
            schema.get("properties").is_some(),
            "{name} inputSchema must have properties"
        );

        // All schemas must have required array (even if empty)
        assert!(
            schema.get("required").and_then(|v| v.as_array()).is_some(),
            "{name} inputSchema must have required array"
        );
    }
}

#[test]
fn tools_descriptions_are_not_empty() {
    let list = mcp::tools::list();
    let tools = list
        .get("tools")
        .and_then(|v| v.as_array())
        .expect("tools must be an array");

    for tool in tools {
        let name = tool
            .get("name")
            .and_then(|v| v.as_str())
            .expect("tool must have name");

        let description = tool
            .get("description")
            .and_then(|v| v.as_str())
            .expect(&format!("{name} must have description"));

        assert!(
            description.len() >= 10,
            "{name} description is too short: {description}"
        );
    }
}

#[test]
fn tools_names_are_snake_case() {
    let list = mcp::tools::list();
    let tools = list
        .get("tools")
        .and_then(|v| v.as_array())
        .expect("tools must be an array");

    for tool in tools {
        let name = tool
            .get("name")
            .and_then(|v| v.as_str())
            .expect("tool must have name");

        assert!(
            name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "tool name '{name}' must be snake_case"
        );
        assert!(
            !name.starts_with('_') && !name.ends_with('_'),
            "tool name '{name}' must not start/end with underscore"
        );
    }
}
