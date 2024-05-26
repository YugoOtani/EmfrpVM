use anyhow::bail;
use anyhow::Result;

use crate::{ast::*, grammer::*};
pub struct ParserWrapper {
    p_type: TypeDefParser,
    p_var: VarDefParser,
    p_exp: ExpParser,
    p_prog: ProgramParser,
    res: Vec<Def>,
}
impl ParserWrapper {
    pub fn new() -> Self {
        Self {
            p_type: TypeDefParser::new(),
            p_var: VarDefParser::new(),
            p_exp: ExpParser::new(),
            p_prog: ProgramParser::new(),
            res: vec![],
        }
    }
    pub fn parse_line(&mut self, input: &str) -> Result<Program> {
        if let Ok(res) = self.p_exp.parse(input) {
            Ok(Program::Exp(res))
        } else if let Ok(res) = self.p_var.parse(input) {
            Ok(Program::Def(vec![Def::Var(res)]))
        } else if let Ok(res) = self.p_type.parse(input) {
            Ok(Program::Def(vec![Def::Type(res)]))
        } else {
            bail!("parse err")
        }
    }
    pub fn parse_program(&self, input: &str) -> Result<Program> {
        match self.p_prog.parse(input) {
            Ok(res) => Ok(res),
            Err(e) => {
                println!("{e}");
                bail!("parse error")
            }
        }
    }
    pub fn parse_one_of_lines(&mut self, input: &str) -> Result<()> {
        if let Ok(res) = self.p_var.parse(input) {
            self.res.push(Def::Var(res));
            Ok(())
        } else if let Ok(res) = self.p_type.parse(input) {
            self.res.push(Def::Type(res));
            Ok(())
        } else if let Ok(_) = self.p_exp.parse(input) {
            bail!("expression is not allowed here")
        } else {
            bail!("parse error")
        }
    }
    pub fn get_result(&mut self) -> Program {
        Program::Def(std::mem::take(&mut self.res))
    }
}
