pub struct Greeter;

impl Greeter {
    pub fn greet(&self, name: &str) -> String {
        let name = self.prepare_name(name);
        self.message(&name)
    }

    fn prepare_name(&self, name: &str) -> String {
        name.trim().to_owned()
    }

    fn message(&self, name: &str) -> String {
        format!("Hello, {name}!")
    }
}
