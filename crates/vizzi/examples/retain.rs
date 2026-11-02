//! JSON-lines adapter for the optional owned-copy retention probe.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("retain requires a configuration path")?;
    let result = vizzi::retention::run(&std::fs::read(path)?, &mut |event| println!("{event}"))?;
    println!("{}", serde_json::json!({"report":result}));
    Ok(())
}
