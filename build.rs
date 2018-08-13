extern crate lalrpop;

use lalrpop::Configuration;

fn main() {
    let mut c = Configuration::new();
    c.use_cargo_dir_conventions();

    c.process_current_dir().unwrap();
}
