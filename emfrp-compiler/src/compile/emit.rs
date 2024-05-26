use std::cmp::max;
use std::vec;

use super::compile_common::*;
use super::typed_ast::*;
use crate::ast::Id;
use crate::insn::*;

impl Compiler {
    fn push_insn(&mut self, insn: Insn) {
        self.codes.push(insn)
    }
    fn insn_clear(&mut self) -> Vec<Insn> {
        std::mem::take(&mut self.codes)
    }
    pub(super) fn emit_code_def_nodes(
        &mut self,
        defs: &Vec<TVarDef>,
    ) -> CResult<Vec<(usize, Vec<Insn>)>> {
        assert_eq!(self.codes.len(), 0);
        let mut ret = vec![];
        for def in defs {
            if let TVarDef::Node { name, init: _, val } = def {
                assert_eq!(self.codes.len(), 0);
                let i = self.node_offset(name).unwrap();
                if self.symbol_table.len() != 0 {
                    panic!(
                        "symbol table len is not zero({}) at the beginning of emit_code_entry",
                        self.symbol_table.len()
                    );
                }
                self.symbol_table.push((None, false)); // next ip (at runtime)
                val.emit_code_entry(self)?;
                self.symbol_table.pop();
                if self.symbol_table.len() != 0 {
                    panic!(
                        "symbol table len is not zero({}) at the end of emit_code_entry",
                        self.symbol_table.len()
                    );
                }
                if let Some(x) = self.node_info[i].output_offset {
                    self.push_insn(Insn::OutputAction(UnsignedNum::U8(x)))
                }
                if self.node_info[i].typ.is_obj_type() {
                    self.push_insn(Insn::EndUpdateNodeObj(UnsignedNum::from_usize(i).unwrap()));
                } else {
                    self.push_insn(Insn::EndUpdateNode(UnsignedNum::from_usize(i).unwrap()));
                }

                ret.push((i, self.insn_clear()));
            }
        }
        Ok(ret)
    }
    pub(super) fn emit_code_def_funcs(
        &mut self,
        defs: &Vec<TVarDef>,
    ) -> CResult<Vec<(usize, Vec<Insn>)>> {
        assert_eq!(self.codes.len(), 0);
        let mut ret = vec![];
        for def in defs {
            if let TVarDef::Func { name, params, body } = def {
                assert!(self.symbol_table.len() == 0);
                if params.len() > u8::MAX as usize {
                    return Err(CompileErr::TooManyLocalVars);
                }
                let mut drop_list = vec![];
                for (i, (id, t)) in params.iter().enumerate() {
                    self.symbol_table.push((Some(id.clone()), t.is_obj_type()));
                    if t.is_obj_type() {
                        drop_list.push(i);
                    }
                }
                body.emit_code_entry(self)?;
                for i in drop_list {
                    self.push_insn(Insn::DropLocalObj(SignedNum::from_usize(i).unwrap()))
                }
                self.push_insn(Insn::Return);
                let i = self.func_offset(name).unwrap();
                assert_eq!(self.symbol_table.len(), params.len());
                self.symbol_table.clear();
                ret.push((i, self.insn_clear()))
            }
        }
        Ok(ret)
    }
    pub(super) fn emit_code_init(&mut self, defs: &Vec<TVarDef>) -> CResult<Vec<Insn>> {
        assert_eq!(self.codes.len(), 0);
        for def in defs {
            match def {
                TVarDef::Node { name, init, val: _ } => {
                    let i = self.node_offset(name).unwrap();
                    let is_new = self.node_info[i].is_new;
                    let is_obj = self.node_info[i].typ.is_obj_type();
                    if let Some(e) = init {
                        assert!(self.symbol_table.len() == 0);
                        e.emit_code_entry(self)?;
                        assert!(self.symbol_table.len() == 0);
                        if is_obj && !is_new {
                            self.push_insn(Insn::SetNodeRef(UnsignedNum::from_usize(i).unwrap()))
                        } else {
                            self.push_insn(Insn::SetNode(UnsignedNum::from_usize(i).unwrap()))
                        }
                    }
                }
                TVarDef::Data { name, val } => {
                    let i = self.data_offset(name).unwrap();
                    assert!(self.symbol_table.len() == 0);
                    val.emit_code_entry(self)?;
                    assert!(self.symbol_table.len() == 0);
                    if val.get_type().is_obj_type() {
                        self.codes
                            .push(Insn::SetDataRef(UnsignedNum::from_usize(i).unwrap()))
                    } else {
                        self.codes
                            .push(Insn::SetData(UnsignedNum::from_usize(i).unwrap()))
                    }
                }
                _ => continue,
            }
        }
        let mut ret = self.insn_clear();
        ret.push(Insn::Halt);
        Ok(ret)
    }

    pub(super) fn emit_code_exp(&mut self, texp: &TExp) -> CResult<Vec<Insn>> {
        assert!(self.codes.len() == 0);
        assert!(self.symbol_table.len() == 0);
        texp.emit_code_entry(self)?;
        assert!(self.symbol_table.len() == 0);
        Ok(self.insn_clear())
    }
}

impl TExp {
    fn local_var_size(&self) -> usize {
        match self {
            TExp::Match(mat, branches) => {
                let mut ret = mat.local_var_size();
                for b in branches {
                    ret = max(ret, 1 + b.local_var_size())
                }
                if let Type::User(..) = mat.get_type() {
                    ret += 1; // for tag
                }
                ret
            }
            TExp::If { cond, then, els } => max(
                max(cond.local_var_size(), then.local_var_size()),
                els.local_var_size(),
            ),
            TExp::Term(t) => t.local_var_size(),
            TExp::Block(b) => b.local_var_size(),
        }
    }
}
impl TTerm {
    fn local_var_size(&self) -> usize {
        match self {
            TLogical::And(a, b) => max(a.local_var_size(), b.local_var_size()),
            TLogical::Or(a, b) => max(a.local_var_size(), b.local_var_size()),
            TLogical::BitWise(a) => a.local_var_size(),
        }
    }
}
impl TBitWise {
    fn local_var_size(&self) -> usize {
        match self {
            TBitWise::And(a, b) => max(a.local_var_size(), b.local_var_size()),
            TBitWise::Or(a, b) => max(a.local_var_size(), b.local_var_size()),
            TBitWise::Xor(a, b) => max(a.local_var_size(), b.local_var_size()),
            TBitWise::Comp(a) => a.local_var_size(),
        }
    }
}
impl TComp {
    fn local_var_size(&self) -> usize {
        match self {
            TComp::Eq(a, b) => max(a.local_var_size(), b.local_var_size()),
            TComp::Neq(a, b) => max(a.local_var_size(), b.local_var_size()),
            TComp::Comp2(a) => a.local_var_size(),
        }
    }
}
impl TComp2 {
    fn local_var_size(&self) -> usize {
        match self {
            TComp2::Leq(a, b) => max(a.local_var_size(), b.local_var_size()),
            TComp2::Ls(a, b) => max(a.local_var_size(), b.local_var_size()),
            TComp2::Geq(a, b) => max(a.local_var_size(), b.local_var_size()),
            TComp2::Gt(a, b) => max(a.local_var_size(), b.local_var_size()),
            TComp2::Shift(a) => a.local_var_size(),
        }
    }
}
impl TShift {
    fn local_var_size(&self) -> usize {
        match self {
            TShift::Left(a, b) => max(a.local_var_size(), b.local_var_size()),
            TShift::Right(a, b) => max(a.local_var_size(), b.local_var_size()),
            TShift::Add(a) => a.local_var_size(),
        }
    }
}
impl TAdd {
    fn local_var_size(&self) -> usize {
        match self {
            TAdd::Plus(a, b) => max(a.local_var_size(), b.local_var_size()),
            TAdd::Minus(a, b) => max(a.local_var_size(), b.local_var_size()),
            TAdd::Factor(a) => a.local_var_size(),
        }
    }
}
impl TFactor {
    fn local_var_size(&self) -> usize {
        match self {
            TFactor::Mul(a, b) => max(a.local_var_size(), b.local_var_size()),
            TFactor::Div(a, b) => max(a.local_var_size(), b.local_var_size()),
            TFactor::Mod(a, b) => max(a.local_var_size(), b.local_var_size()),
            TFactor::Unary(a) => a.local_var_size(),
        }
    }
}
impl TUnary {
    fn local_var_size(&self) -> usize {
        match self {
            TUnary::Not(a) => a.local_var_size(),
            TUnary::Minus(a) => a.local_var_size(),
            TUnary::Primary(a) => a.local_var_size(),
        }
    }
}
impl TPrimary {
    fn local_var_size(&self) -> usize {
        match self {
            TPrimary::Int(_) => 0,
            TPrimary::Bool(_) => 0,
            TPrimary::Exp(e) => e.local_var_size(),
            TPrimary::Id(_, _) => 0,
            TPrimary::Last(_, _) => 0,
            TPrimary::Variant(_, _, exps)
            | TPrimary::FnCall(_, _, exps)
            | TPrimary::Tuple(exps, _) => {
                let mut ret = 0;
                for exp in exps {
                    ret = max(ret, exp.local_var_size())
                }
                ret
            }
        }
    }
}
impl TBlock {
    fn local_var_size(&self) -> usize {
        let TBlock { stmt, exp } = self;
        let mut ret = 0;
        for (i, TStmt { id: _, val }) in stmt.iter().enumerate() {
            ret = max(ret, i + val.local_var_size())
        }
        max(ret, stmt.len() + exp.local_var_size())
    }
}
impl TPattern {
    fn local_var_size(&self) -> usize {
        match self {
            TPattern::Int(_) => 0,
            TPattern::Id(_, _) => 1,
            TPattern::Variant(_, pats) | TPattern::Tuple(pats) => {
                pats.iter().fold(0, |acc, p| acc + p.local_var_size())
            }
            TPattern::Bool(_) => 0,
            TPattern::None => 0,
        }
    }
}
impl TBranch {
    fn local_var_size(&self) -> usize {
        let TBranch { pat, exp } = self;
        pat.local_var_size() + exp.local_var_size()
    }
}
fn emit_alloc_local(size: usize, c: &mut Compiler) -> CResult<()> {
    match size {
        0 => (),
        n if n <= std::u8::MAX as usize => c.push_insn(Insn::AllocLocal(UnsignedNum::U8(n as u8))),
        _ => return Err(CompileErr::TooManyLocalVars),
    }
    Ok(())
}
fn emit_pop_local(size: usize, c: &mut Compiler) {
    match size {
        0 => (),
        n => c.push_insn(Insn::Pop(UnsignedNum::U8(n as u8))),
    }
}
fn emit_push_top_on_local_stack(var: Option<Id>, b: IsObjType, c: &mut Compiler) {
    let i = c.symbol_table.len();
    c.push_insn(Insn::SetLocal(SignedNum::from_usize(i).unwrap()));
    c.symbol_table.push((var, b));
    if c.symbol_table.len() > c.local_len {
        panic!(
            "symbol table : {:?}, local len : {}",
            c.symbol_table, c.local_len
        );
    }
}

impl TExp {
    fn emit_code_entry(&self, c: &mut Compiler) -> CResult<()> {
        let tbl_len = c.symbol_table.len();
        let local_len = self.local_var_size();
        if local_len == 0 {
            self.emit_code_body(c)?;
            return Ok(());
        }
        c.local_len = local_len + tbl_len + 1; // 1 is a return value
                                               //prepare space for return value
        c.symbol_table.push((None, false));
        // prepare space for local variables necessary to emit body
        emit_alloc_local(local_len + 1, c)?;
        // emit body
        self.emit_code_body(c)?;
        // save result and drop local vars
        c.push_insn(Insn::SetLocal(SignedNum::from_usize(tbl_len).unwrap()));
        emit_pop_local(local_len, c);

        //
        c.symbol_table.pop();
        c.local_len = tbl_len;
        if c.symbol_table.len() != tbl_len {
            panic!("{:?}", c.symbol_table)
        }
        Ok(())
    }
    fn emit_code_body(&self, c: &mut Compiler) -> CResult<()> {
        match self {
            TExp::Match(exp, branches) => {
                let match_exp_offset = c.symbol_table.len() as i32;
                // push match exp to local stack
                c.symbol_table.push((None, exp.get_type().is_obj_type()));
                exp.emit_code(c)?;
                c.push_insn(Insn::SetLocal(SignedNum::from_i32(match_exp_offset)));

                match exp.get_type() {
                    Type::Int => {
                        let mut gotoend_offset = vec![];
                        for TBranch { pat, exp } in branches {
                            match pat {
                                TPattern::Int(i) => {
                                    c.push_insn(Insn::GetLocal(SignedNum::from_i32(
                                        match_exp_offset,
                                    )));
                                    c.push_insn(Insn::Int(SignedNum::from_i32(*i)));
                                    c.push_insn(Insn::Eq);
                                    c.push_insn(Insn::Placeholder);
                                    let i0 = c.codes.len();
                                    exp.emit_code_body(c)?;
                                    gotoend_offset.push(c.codes.len());
                                    c.codes.push(Insn::J32(0)); //0 is placeholder
                                    let i1 = c.codes.len();
                                    c.codes[i0 - 1] =
                                        Insn::jne(bytecode_len(&c.codes[i0..i1]) as i32);
                                }
                                TPattern::Id(_, id) => {
                                    c.push_insn(Insn::GetLocal(SignedNum::from_i32(
                                        match_exp_offset,
                                    )));
                                    emit_push_top_on_local_stack(Some(id.clone()), false, c);
                                    exp.emit_code_body(c)?;
                                    gotoend_offset.push(c.codes.len());
                                    c.codes.push(Insn::J32(0)); //0 is placeholder
                                    c.symbol_table.pop();
                                }
                                TPattern::Variant(_, _) => {
                                    panic!("typecheck")
                                }
                                TPattern::Bool(_) => panic!("typecheck"),
                                TPattern::Tuple(_) => panic!("typecheck"),
                                TPattern::None => {
                                    exp.emit_code_body(c)?;
                                    gotoend_offset.push(c.codes.len());
                                    c.codes.push(Insn::J32(0)); //0 is placeholder
                                }
                            }
                        }
                        c.push_insn(Insn::Abort);
                        let end = c.codes.len();
                        for st in gotoend_offset {
                            c.codes[st] = Insn::J32(bytecode_len(&c.codes[st + 1..end]) as i32);
                        }
                    }
                    Type::Bool => {
                        let mut gotoend_offset = vec![];
                        for TBranch { pat, exp } in branches {
                            match pat {
                                TPattern::Int(_) => {
                                    panic!("typecheck")
                                }
                                TPattern::Id(_, id) => {
                                    c.push_insn(Insn::GetLocal(SignedNum::from_i32(
                                        match_exp_offset,
                                    )));
                                    emit_push_top_on_local_stack(Some(id.clone()), false, c);
                                    exp.emit_code_body(c)?;
                                    gotoend_offset.push(c.codes.len());
                                    c.codes.push(Insn::J32(0)); //0 is placeholder
                                    c.symbol_table.pop();
                                }
                                TPattern::Variant(_, _) => {
                                    panic!("typecheck")
                                }
                                TPattern::Bool(b) => {
                                    c.push_insn(Insn::GetLocal(SignedNum::from_i32(
                                        match_exp_offset,
                                    )));
                                    c.push_insn(Insn::Placeholder);
                                    let i0 = c.codes.len();
                                    exp.emit_code_body(c)?;
                                    gotoend_offset.push(c.codes.len());
                                    c.codes.push(Insn::J32(0)); //0 is placeholder
                                    let i1 = c.codes.len();
                                    c.codes[i0 - 1] = if *b {
                                        Insn::jne(bytecode_len(&c.codes[i0..i1]) as i32)
                                    } else {
                                        Insn::je(bytecode_len(&c.codes[i0..i1]) as i32)
                                    };
                                }
                                TPattern::Tuple(_) => panic!("typecheck"),
                                TPattern::None => {
                                    exp.emit_code_body(c)?;
                                    gotoend_offset.push(c.codes.len());
                                    c.codes.push(Insn::J32(0)); //0 is placeholder
                                }
                            }
                        }
                        let end = c.codes.len();
                        for st in gotoend_offset {
                            c.codes[st] = Insn::J32(bytecode_len(&c.codes[st + 1..end]) as i32);
                        }
                    }
                    Type::User(_, _) => {
                        let mut gotoend_offset = vec![];
                        c.push_insn(Insn::GetLocal(SignedNum::from_i32(match_exp_offset)));
                        c.push_insn(Insn::ObjTag);
                        let tag_offset = c.symbol_table.len();
                        emit_push_top_on_local_stack(None, false, c); //tag space
                        for TBranch { pat, exp } in branches {
                            match pat {
                                TPattern::Int(_) => panic!("typecheck"),
                                TPattern::Bool(_) => panic!("typecheck"),
                                TPattern::Tuple(_) => panic!("typecheck"),
                                TPattern::None => {
                                    exp.emit_code_body(c)?;
                                    gotoend_offset.push(c.codes.len());
                                    c.codes.push(Insn::J32(0)); //0 is placeholder
                                }
                                TPattern::Id(_, id) => {
                                    c.push_insn(Insn::GetLocal(SignedNum::from_i32(
                                        match_exp_offset,
                                    )));
                                    emit_push_top_on_local_stack(Some(id.clone()), true, c);
                                    exp.emit_code_body(c)?;
                                    c.symbol_table.pop();
                                    gotoend_offset.push(c.codes.len());
                                    c.codes.push(Insn::J32(0)); //0 is placeholder
                                }
                                TPattern::Variant(tag, pats) => {
                                    c.push_insn(Insn::GetLocal(
                                        SignedNum::from_usize(tag_offset).unwrap(),
                                    ));
                                    c.push_insn(Insn::Int(SignedNum::from_usize(*tag).unwrap()));
                                    c.push_insn(Insn::Eq);
                                    c.push_insn(Insn::Placeholder);
                                    let i0 = c.codes.len();
                                    let mut local_cnt = 0;
                                    for (i, pat) in pats.iter().enumerate() {
                                        match pat {
                                            TPattern::Id(t, id) => {
                                                local_cnt += 1;
                                                c.push_insn(Insn::GetLocal(SignedNum::from_i32(
                                                    match_exp_offset,
                                                )));
                                                c.push_insn(Insn::ObjField(
                                                    UnsignedNum::from_usize(i).unwrap(),
                                                ));
                                                emit_push_top_on_local_stack(
                                                    Some(id.clone()),
                                                    t.is_obj_type(),
                                                    c,
                                                );
                                            }
                                            TPattern::Variant(_, _) => todo!(),
                                            TPattern::Bool(_) => todo!(),
                                            TPattern::Int(_) => todo!(),
                                            TPattern::Tuple(_) => todo!(),
                                            TPattern::None => {
                                                continue;
                                            }
                                        }
                                    }
                                    exp.emit_code_body(c)?;
                                    for _ in 0..local_cnt {
                                        c.symbol_table.pop();
                                    }
                                    gotoend_offset.push(c.codes.len());
                                    c.codes.push(Insn::J32(0)); //0 is placeholder
                                    let i1 = c.codes.len();
                                    c.codes[i0 - 1] =
                                        Insn::jne(bytecode_len(&c.codes[i0..i1]) as i32);
                                }
                            }
                        }
                        c.push_insn(Insn::Abort);
                        c.symbol_table.pop(); // pop tag
                        let end = c.codes.len();
                        for st in gotoend_offset {
                            c.codes[st] = Insn::J32(bytecode_len(&c.codes[st + 1..end]) as i32);
                        }
                    }
                    Type::Tuple(_) => {
                        let mut gotoend_offset = vec![];
                        for TBranch { pat, exp } in branches {
                            match pat {
                                TPattern::Int(_) => panic!("typecheck"),
                                TPattern::Bool(_) => panic!("typecheck"),
                                TPattern::Tuple(pats) => {
                                    let mut local_cnt = 0;
                                    for (i, pat) in pats.iter().enumerate() {
                                        match pat {
                                            TPattern::Id(t, id) => {
                                                local_cnt += 1;
                                                c.push_insn(Insn::GetLocal(SignedNum::from_i32(
                                                    match_exp_offset,
                                                )));
                                                c.push_insn(Insn::ObjField(
                                                    UnsignedNum::from_usize(i).unwrap(),
                                                ));
                                                emit_push_top_on_local_stack(
                                                    Some(id.clone()),
                                                    t.is_obj_type(),
                                                    c,
                                                );
                                            }
                                            TPattern::Variant(_, _) => todo!(),
                                            TPattern::Bool(_) => todo!(),
                                            TPattern::Int(_) => todo!(),
                                            TPattern::Tuple(_) => todo!(),
                                            TPattern::None => {
                                                continue;
                                            }
                                        }
                                    }
                                    exp.emit_code_body(c)?;
                                    for _ in 0..local_cnt {
                                        c.symbol_table.pop();
                                    }
                                    gotoend_offset.push(c.codes.len());
                                    c.codes.push(Insn::J32(0)); //0 is placeholder
                                }
                                TPattern::None => todo!(),
                                TPattern::Id(t, id) => {
                                    c.push_insn(Insn::GetLocal(SignedNum::from_i32(
                                        match_exp_offset,
                                    )));
                                    emit_push_top_on_local_stack(
                                        Some(id.clone()),
                                        t.is_obj_type(),
                                        c,
                                    );
                                    exp.emit_code_body(c)?;
                                    c.symbol_table.pop();
                                    gotoend_offset.push(c.codes.len());
                                    c.codes.push(Insn::J32(0)); //0 is placeholder
                                }
                                TPattern::Variant(..) => {
                                    panic!("typecheck")
                                }
                            }
                        }
                        c.push_insn(Insn::Abort);
                        let end = c.codes.len();
                        for st in gotoend_offset {
                            c.codes[st] = Insn::J32(bytecode_len(&c.codes[st + 1..end]) as i32);
                        }
                    }
                };
                c.symbol_table.pop(); // pop match exp
                if exp.get_type().is_obj_type() {
                    c.push_insn(Insn::DropLocalObj(SignedNum::from_i32(match_exp_offset)));
                }
                Ok(())
            }
            TExp::If { cond, then, els } => {
                cond.emit_code(c)?;
                c.push_insn(Insn::Placeholder);
                let i0 = c.codes.len();
                els.emit_code_body(c)?;
                c.push_insn(Insn::Placeholder);
                let i1 = c.codes.len();
                then.emit_code_body(c)?;
                let i2 = c.codes.len();
                c.codes[i1 as usize - 1] = Insn::j(bytecode_len(&c.codes[i1..i2]) as i32);

                c.codes[i0 as usize - 1] = Insn::je(bytecode_len(&c.codes[i0..i1]) as i32);

                Ok(())
            }
            TExp::Term(a) => a.emit_code(c),
            TExp::Block(b) => b.emit_code_body(c),
        }
    }
}
impl TBlock {
    fn emit_code_body(&self, c: &mut Compiler) -> CResult<()> {
        let TBlock { stmt, exp } = self;
        let mut drop_list = vec![];
        for TStmt { id, val } in stmt {
            val.emit_code_body(c)?;
            drop_list.push(c.symbol_table.len());
            emit_push_top_on_local_stack(Some(id.clone()), val.get_type().is_obj_type(), c);
        }
        exp.emit_code_body(c)?;
        for _ in 0..stmt.len() {
            c.symbol_table.pop();
        }
        for i in drop_list {
            c.push_insn(Insn::DropLocalObj(SignedNum::from_i32(i as i32)))
        }

        Ok(())
    }
}
impl TTerm {
    pub(super) fn emit_code(&self, c: &mut Compiler) -> CResult<()> {
        match self {
            TLogical::And(a, b) => {
                a.emit_code(c)?;
                c.push_insn(Insn::Placeholder);
                let i0 = c.codes.len();
                b.emit_code(c)?;
                c.push_insn(Insn::J1);
                let i1 = c.codes.len();
                c.push_insn(Insn::PushFalse);
                c.codes[i0 - 1] = Insn::jne(bytecode_len(&c.codes[i0..i1]) as i32);

                Ok(())
            }
            TLogical::Or(a, b) => {
                a.emit_code(c)?;
                c.push_insn(Insn::Placeholder);
                let i0 = c.codes.len();
                b.emit_code(c)?;
                c.push_insn(Insn::J1);
                let i1 = c.codes.len();
                c.push_insn(Insn::PushTrue);
                c.codes[i0 - 1] = Insn::je(bytecode_len(&c.codes[i0..i1]) as i32);
                Ok(())
            }
            TLogical::BitWise(a) => a.emit_code(c),
        }
    }
}
impl TBitWise {
    pub(super) fn emit_code(&self, c: &mut Compiler) -> CResult<()> {
        match self {
            TBitWise::And(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::BitAnd);
                Ok(())
            }
            TBitWise::Or(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::BitOr);
                Ok(())
            }
            TBitWise::Xor(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::BitXor);
                Ok(())
            }
            TBitWise::Comp(a) => a.emit_code(c),
        }
    }
}
impl TComp {
    pub(super) fn emit_code(&self, c: &mut Compiler) -> CResult<()> {
        match self {
            TComp::Eq(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::Eq);
                Ok(())
            }
            TComp::Neq(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::Neq);
                Ok(())
            }
            TComp::Comp2(a) => a.emit_code(c),
        }
    }
}
impl TComp2 {
    pub(super) fn emit_code(&self, c: &mut Compiler) -> CResult<()> {
        match self {
            TComp2::Leq(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::Leq);
                Ok(())
            }
            TComp2::Ls(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::Ls);
                Ok(())
            }
            TComp2::Geq(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::Geq);
                Ok(())
            }
            TComp2::Gt(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::Gt);
                Ok(())
            }
            TComp2::Shift(a) => a.emit_code(c),
        }
    }
}
impl TShift {
    pub(super) fn emit_code(&self, c: &mut Compiler) -> CResult<()> {
        match self {
            TShift::Left(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::ShiftL);
                Ok(())
            }
            TShift::Right(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::ShiftR);
                Ok(())
            }
            TShift::Add(a) => a.emit_code(c),
        }
    }
}
impl TAdd {
    pub(super) fn emit_code(&self, c: &mut Compiler) -> CResult<()> {
        match self {
            TAdd::Plus(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::Add);
                Ok(())
            }
            TAdd::Minus(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::Sub);
                Ok(())
            }
            TAdd::Factor(f) => f.emit_code(c),
        }
    }
}
impl TFactor {
    pub(super) fn emit_code(&self, c: &mut Compiler) -> CResult<()> {
        match self {
            TFactor::Mul(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::Mul);
            }
            TFactor::Div(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::Div);
            }
            TFactor::Mod(a, b) => {
                a.emit_code(c)?;
                b.emit_code(c)?;
                c.push_insn(Insn::Mod);
            }
            TFactor::Unary(a) => return a.emit_code(c),
        }
        Ok(())
    }
}
impl TUnary {
    pub(super) fn emit_code(&self, c: &mut Compiler) -> CResult<()> {
        match self {
            TUnary::Not(a) => {
                a.emit_code(c)?;
                c.push_insn(Insn::Not);
            }
            TUnary::Minus(a) => {
                a.emit_code(c)?;
                c.push_insn(Insn::Minus);
            }
            TUnary::Primary(a) => return a.emit_code(c),
        }
        Ok(())
    }
}
impl TPrimary {
    pub(super) fn emit_code(&self, c: &mut Compiler) -> CResult<()> {
        match self {
            TPrimary::Int(i) => c.push_insn(Insn::Int(SignedNum::from_i32(*i))),
            TPrimary::Bool(b) => c.push_insn(if *b { Insn::PushTrue } else { Insn::PushFalse }),
            TPrimary::Exp(e) => return e.emit_code_body(c),
            TPrimary::Id(id, _) => {
                for i in (0..c.symbol_table.len()).rev() {
                    if let (Some(id2), is_obj) = &c.symbol_table[i] {
                        if id == id2 {
                            if *is_obj {
                                c.push_insn(Insn::GetLocalRef(SignedNum::from_usize(i).unwrap()));
                            } else {
                                c.push_insn(Insn::GetLocal(SignedNum::from_usize(i).unwrap()));
                            }
                            return Ok(());
                        }
                    }
                }
                if let Some(i) = c.node_offset(id) {
                    if c.node_info[i].typ.is_obj_type() {
                        c.push_insn(Insn::GetNodeRef(UnsignedNum::from_usize(i).unwrap()))
                    } else {
                        c.push_insn(Insn::GetNode(UnsignedNum::from_usize(i).unwrap()))
                    }
                } else if let Some(i) = c.data_offset(id) {
                    if c.data_info[i].typ.is_obj_type() {
                        c.push_insn(Insn::GetDataRef(UnsignedNum::from_usize(i).unwrap()))
                    } else {
                        c.push_insn(Insn::GetData(UnsignedNum::from_usize(i).unwrap()))
                    }
                } else {
                    panic!("typecheck")
                }
            }
            TPrimary::Last(id, t) => {
                if let Some(i) = c.node_offset(id) {
                    if c.node_info[i].is_new && !c.node_info[i].has_value {
                        return Err(CompileErr::InvalidAtLast);
                    }
                    let last_offset = c.atlast_manager.runtime_offset(i);
                    // if last_offset is none, x@last is referenced
                    // only by node x, so x@last's value is same as x's
                    // if last_offset is some, x@last is referenced by other node,
                    // so x@last can be accessed by last offset
                    let insn = match (last_offset, t.is_obj_type()) {
                        (Some(last_i), true) => {
                            Insn::GetLastRef(UnsignedNum::from_u32(last_i as u32))
                        }
                        (Some(last_i), false) => {
                            Insn::GetLast(UnsignedNum::from_u32(last_i as u32))
                        }
                        (None, true) => Insn::GetNodeRef(UnsignedNum::from_u32(i as u32)),
                        (None, false) => Insn::GetNode(UnsignedNum::from_u32(i as u32)),
                    };
                    c.push_insn(insn);
                } else {
                    panic!("typecheck")
                }
            }
            TPrimary::FnCall(id, _, e) => {
                for e in e {
                    e.emit_code_body(c)?;
                }
                let f = c.func_offset(id).unwrap();
                c.push_insn(Insn::Call(
                    e.len() as u8,
                    UnsignedNum::from_usize(f).unwrap(),
                ));
            }
            TPrimary::Variant(vname, _, exps) => {
                let (t, tag, _) = c.get_type_from_variant(&vname).unwrap();
                let max_entry = max_entry(t);
                let mut objbit = vec![];
                for e in exps {
                    objbit.push(e.get_type().is_obj_type());
                    e.emit_code_body(c)?;
                }
                let header = ObjHeader::new(tag as u32, &objbit, exps.len() as u32);
                assert!(max_entry <= u8::MAX as usize);
                c.push_insn(Insn::AllocObj(UnsignedNum::U8(max_entry as u8), header))
            }
            TPrimary::Tuple(exps, _) => {
                let mut objbit = vec![];
                for e in exps {
                    objbit.push(e.get_type().is_obj_type());
                    e.emit_code_body(c)?;
                }
                let header = ObjHeader::new(1, &objbit, exps.len() as u32);
                assert!(exps.len() <= u8::MAX as usize);
                c.push_insn(Insn::AllocObj(UnsignedNum::U8(exps.len() as u8), header))
            }
        }
        Ok(())
    }
}
