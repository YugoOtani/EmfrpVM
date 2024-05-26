mod ast;
mod parser;
pub mod compile {
    pub mod compile;
    pub mod compile_common;
    mod dependency;
    mod emit;
    mod typecheck;
    mod typed_ast;
    mod typeinfer;
}
pub mod insn;
pub mod repl;
lalrpop_mod!(grammer);
use compile::compile_common::Type;
use lalrpop_util::lalrpop_mod;

use crate::repl::*;

use std::io::*;
const UART_FILE: &str = "/dev/cu.usbserial-0001";
//const UART_FILE: &str = "/dev/cu.usbmodem142101";
const BAUD_RATE: u32 = 115200;
const DEBUG: bool = true;

fn main() {
    let mut repl = match Repl::new() {
        Ok(r) => r,
        Err(s) => {
            println!("{s}");
            return;
        }
    };

    repl.add_input_node("gpio16", Type::Bool);
    repl.add_output_node("gpio5", Type::Bool);
    //repl.add_output_node("led", Type::Bool);

    repl.run();
}
