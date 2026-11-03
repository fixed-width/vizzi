//! JSON-lines adapter for source-only storage/retention measurement.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("source_storage requires a configuration path")?;
    let report =
        vizzi::source_storage_probe::run(&std::fs::read(path)?, &mut |event| println!("{event}"))?;
    println!("{}", serde_json::json!({"report":report}));
    Ok(())
}
