#![forbid(unsafe_code)]

pub fn run_once(accepted: bool, delivered: &mut Vec<u8>) -> Result<(), &'static str> {
    validate(accepted)?;
    deliver(delivered);
    Ok(())
}

pub fn validate(accepted: bool) -> Result<(), &'static str> {
    if accepted { Ok(()) } else { Err("rejected") }
}

pub fn deliver(delivered: &mut Vec<u8>) {
    delivered.push(1);
}
