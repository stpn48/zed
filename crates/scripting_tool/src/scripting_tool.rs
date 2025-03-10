mod session;

use project::Project;
use session::*;

use assistant_tool::{Tool, ToolRegistry};
use gpui::{App, AppContext as _, Entity, Task};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;

pub fn init(cx: &App) {
    let registry = ToolRegistry::global(cx);
    registry.register_tool(ScriptingTool);
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ScriptingToolInput {
    lua_script: String,
}

struct ScriptingTool;

impl Tool for ScriptingTool {
    fn name(&self) -> String {
        "lua-interpreter".into()
    }

    fn description(&self) -> String {
        include_str!("scripting_tool_description.txt").into()
    }

    fn input_schema(&self) -> serde_json::Value {
        let schema = schemars::schema_for!(ScriptingToolInput);
        serde_json::to_value(&schema).unwrap()
    }

    fn run(
        self: Arc<Self>,
        input: serde_json::Value,
        project: Entity<Project>,
        cx: &mut App,
    ) -> Task<anyhow::Result<String>> {
        let input = match serde_json::from_value::<ScriptingToolInput>(input) {
            Err(err) => return Task::ready(Err(err.into())),
            Ok(input) => input,
        };

        // TODO: Store a session per thread
        let session = cx.new(|cx| ScriptSession::new(project, cx));
        let lua_script = input.lua_script;

        let (script_id, script_task) =
            session.update(cx, |session, cx| session.run_script(lua_script, cx));

        cx.spawn(|cx| async move {
            script_task.await;

            let message = session.read_with(&cx, |session, _cx| {
                // Using a id to get the script output seems impractical.
                // Why not just include it in the Task result?
                // This is because we'll later report the script state as it runs,
                // currently not supported by the `Tool` interface.
                session
                    .get(script_id)
                    .output_message_for_llm()
                    .expect("Script shouldn't still be running")
            })?;

            drop(session);
            Ok(message)
        })
    }
}
