use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
struct Message<'a> {
    ecosystem: &'a str,
    value: u32,
}

fn main() -> Result<()> {
    println!(
        "{}",
        serde_json::to_string(&Message {
            ecosystem: "rust",
            value: 38,
        })?
    );
    Ok(())
}
