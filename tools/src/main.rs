use std::env;
use std::process::Command;

use tools_utils::{run_main, Result};

fn main() {
    run_main(main_impl);
}

fn main_impl() -> Result<i32> {
    let tool = env::args_os()
        .nth(1)
        .ok_or_else(|| String::from("Usage: tools [subtool]"))?;
    let tool = tool
        .into_string()
        .map_err(|_| String::from("The subtool name needs to be valid UTF-8"))?;

    let command = format!("tools-{}", tool);
    let mut child = Command::new(&command)
        .args(env::args_os().skip(2))
        .spawn()
        .map_err(|e| format!("Could not execute subcommand {}: {}", command, e))?;

    let result = child
        .wait()
        .map_err(|e| format!("Could no execute subcommand {}: {}", command, e))?;

    Ok(result.code().unwrap_or(0))
}
