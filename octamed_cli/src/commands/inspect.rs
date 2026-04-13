use crate::commands::repl::Command;

pub struct InspectCommand {}
impl InspectCommand {
    pub fn new() -> Self {
        Self {}
    }
}
impl Command for InspectCommand {
    fn help(&self) -> (String, String) {
        return (
            format!("inspect"),
            format!("inspect the headers of the mmd file without loading in any binary data"),
        );
    }

    fn run(&self, mmd: &mut octamed::mmd0::module::OctamedMMD0) {
        println!("{}", mmd.header);
    }
}
