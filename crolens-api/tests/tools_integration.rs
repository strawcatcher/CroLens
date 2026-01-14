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

    for required in [
        "get_account_summary",
        "get_defi_positions",
        "decode_transaction",
        "simulate_transaction",
        "search_contract",
        "construct_swap_tx",
    ] {
        assert!(names.contains(&required), "missing tool: {required}");
    }
}
