//! JSON-lines adapter for the optional native qualification experiment.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("pass a qualification configuration JSON path")?;
    let report =
        vizzi::qualification::run(&std::fs::read(path)?, &mut |event| println!("{event}"))?;
    println!("{}", serde_json::json!({"event":"report", "report":report}));
    Ok(())
}
