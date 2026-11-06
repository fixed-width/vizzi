#![forbid(unsafe_code)]

pub enum Request {
    Apply(u64),
    Probe,
}
pub struct Coordinator;

impl Coordinator {
    pub fn dispatch(&self, request: Request) -> Result<(), &'static str> {
        match request {
            Request::Apply(token) => self.validate(token),
            Request::Probe => Ok(()),
        }
    }

    pub fn validate(&self, token: u64) -> Result<(), &'static str> {
        if token == 0 {
            Err("empty token")
        } else {
            Ok(())
        }
    }

    pub fn pending(&self) -> usize {
        0
    }
}
