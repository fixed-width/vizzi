//! JSON-lines adapter for the optional admission workflow probe.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("admit requires a configuration path")?;
    let report =
        vizzi::admission_probe::run(&std::fs::read(path)?, &mut |event| println!("{event}"))?;
    println!("{}", serde_json::json!({"report":report}));
    Ok(())
}
