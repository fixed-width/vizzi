#![forbid(unsafe_code)]

pub fn check_requested(requested: u64, committed: u64) -> Result<(), &'static str> {
    if requested > committed {
        Err("rejected")
    } else {
        Ok(())
    }
}

pub fn commit_request(requested: u64, committed: u64) -> Result<(), &'static str> {
    check_requested(requested, committed)
}

pub fn describe(requested: u64) -> String {
    format!("request {requested}")
}
