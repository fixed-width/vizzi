//! JSON-lines adapter for the optional mixed working-set probe.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("working_set requires a configuration path")?;
    let report = vizzi::working_set::run(&std::fs::read(path)?, &mut |event| println!("{event}"))?;
    println!("{}", serde_json::json!({"report":report}));
    Ok(())
}
