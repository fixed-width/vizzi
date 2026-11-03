//! JSON-lines adapter for the foreground/cleanup qualification probe.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("configuration path required")?;
    let report = vizzi::foreground_maintenance::run(&std::fs::read(path)?, &mut |event| {
        println!("{event}")
    })?;
    println!("{}", serde_json::json!({"report":report}));
    Ok(())
}
