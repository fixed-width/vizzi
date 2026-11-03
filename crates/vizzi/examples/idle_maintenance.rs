//! JSON-lines adapter for automatic idle cleanup qualification.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("configuration path required")?;
    let report = vizzi::idle_probe::run(&std::fs::read(path)?, &mut |event| println!("{event}"))?;
    println!("{}", serde_json::json!({"report":report}));
    Ok(())
}
