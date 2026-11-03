//! JSON-lines adapter for the optional bounded large-file capture probe.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("stream_capture requires a configuration path")?;
    let report = vizzi::stream_probe::run(&std::fs::read(path)?, &mut |event| println!("{event}"))?;
    println!("{}", serde_json::json!({"report":report}));
    Ok(())
}
