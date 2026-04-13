pub trait Logger {
    fn log(&self, message: &str);
}

pub struct ConsoleLogger;
impl ConsoleLogger {
    pub fn new() -> Self {
        return Self {};
    }
}
impl Logger for ConsoleLogger {
    fn log(&self, message: &str) {
        println!("{}", message);
    }
}
