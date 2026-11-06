#![forbid(unsafe_code)]

pub fn finish(valid: bool, live_pins: &mut usize) -> Result<(), &'static str> {
    let result = validate(valid);
    release(live_pins);
    result
}

pub fn release(live_pins: &mut usize) {
    *live_pins = live_pins.saturating_sub(1);
}

pub fn validate(valid: bool) -> Result<(), &'static str> {
    if valid { Ok(()) } else { Err("rejected") }
}
