use crate::DEBUG;

use super::compile_common::*;
use super::typed_ast::*;
use std::collections::HashSet;
use std::collections::{HashMap, VecDeque};

type NodeOffset = usize;
type RefCnt = usize;
#[derive(Debug, Clone)]
pub(super) struct AtLastManager {
    atlast_info: Vec<(NodeOffset, RefCnt)>,
}
impl AtLastManager {
    pub fn new() -> Self {
        Self {
            atlast_info: vec![],
        }
    }
    pub fn runtime_offset(&self, nd_offset: NodeOffset) -> Option<usize> {
        for (i, &(nd_i, cnt)) in self.atlast_info.iter().enumerate() {
            if nd_i != nd_offset {
                continue;
            }
            if cnt != 0 {
                return Some(i);
            } else {
                return None;
            }
        }
        None
    }
    pub fn remove_atlast_refcnt(&mut self, nd_offset: NodeOffset) {
        for (nd_i, cnt) in self.atlast_info.iter_mut() {
            if *nd_i == nd_offset {
                *cnt -= 1;
                return;
            }
        }
        panic!();
    }
    pub fn add_atlast_refcnt(&mut self, nd_offset: NodeOffset) {
        for (nd_i, cnt) in self.atlast_info.iter_mut() {
            if *nd_i == nd_offset {
                *cnt += 1;
                return;
            }
        }
        for (nd_i, cnt) in self.atlast_info.iter_mut() {
            if *cnt == 0 {
                *cnt = 1;
                *nd_i = nd_offset;
                return;
            }
        }
        self.atlast_info.push((nd_offset, 1))
    }
    pub fn atlast_offset(&mut self) -> Vec<Option<NodeOffset>> {
        let mut ret = vec![];
        for (nd_i, cnt) in &self.atlast_info {
            if *cnt != 0 {
                ret.push(Some(*nd_i))
            } else {
                ret.push(None)
            }
        }
        ret
    }
}

impl Compiler {
    pub(super) fn topological_sort(&self) -> CResult<Vec<usize>> {
        let mut q = VecDeque::new();
        let mut cnt = HashMap::new();
        let mut ret = vec![];
        for (
            i,
            NodeInfo {
                name,
                typ: _,
                prev,
                is_new: _,
                atlast: _,
                output_offset: _,
                has_value: _,
                input_kind: _,
            },
        ) in self.node_info.iter().enumerate()
        {
            if DEBUG {
                assert_eq!(self.node_offset(name), Some(i));
            }

            if prev.is_empty() {
                q.push_back(i);
                cnt.insert(i, 0);
            } else {
                cnt.insert(i, prev.len());
            }
        }

        while let Some(nd) = q.pop_front() {
            // ndがさすノードのカウントを減らす
            for (
                i,
                NodeInfo {
                    name: _,
                    typ: _,
                    prev: pointed,
                    is_new: _,
                    atlast: _,
                    output_offset: _,
                    has_value: _,
                    input_kind: _,
                },
            ) in self.node_info.iter().enumerate()
            {
                if pointed.contains(&nd) {
                    *cnt.get_mut(&i).unwrap() -= 1;
                    if *cnt.get(&i).unwrap() == 0 {
                        q.push_back(i)
                    }
                }
            }
            ret.push(nd);
        }
        if ret.len() == cnt.len() {
            Ok(ret)
        } else {
            Err(CompileErr::CircularRef)
        }
    }
    pub(super) fn add_dependency(&mut self, defs: &Vec<TVarDef>) -> CResult<()> {
        for def in defs {
            match def {
                TVarDef::Node { name, init: _, val } => {
                    let mut st = HashSet::new();
                    let nd_i = self.node_offset(name).unwrap();
                    val.to_dependency(nd_i, &mut st, self)?;
                    self.node_info[nd_i].prev = st;
                }
                TVarDef::Data { .. } | TVarDef::Func { .. } => continue,
            }
        }

        Ok(())
    }
}
impl TPattern {
    fn add_local_variables(&self, c: &mut Compiler) {
        match self {
            TPattern::Int(_) => return,
            TPattern::Id(t, id) => c.symbol_table.push((Some(id.clone()), t.is_obj_type())),
            TPattern::Variant(_, pats) => {
                for pat in pats {
                    pat.add_local_variables(c)
                }
            }
            TPattern::Bool(_) => return,
            TPattern::Tuple(_) => return,
            TPattern::None => return,
        }
    }
}

impl TExp {
    pub(super) fn to_dependency(
        &self,
        nd_i: usize,
        lst: &mut HashSet<usize>,
        c: &mut Compiler,
    ) -> CResult<()> {
        match self {
            TExp::Match(e, branches) => {
                e.to_dependency(nd_i, lst, c)?;
                let i0 = c.symbol_table.len();
                for TBranch { pat, exp: _ } in branches {
                    pat.add_local_variables(c);
                }
                let n_local = c.symbol_table.len() - i0;
                for TBranch { pat: _, exp } in branches {
                    exp.to_dependency(nd_i, lst, c)?;
                }
                for _ in 0..n_local {
                    c.symbol_table.pop();
                }
                Ok(())
            }
            TExp::If { cond, then, els } => {
                cond.to_dependency(nd_i, lst, c)?;
                then.to_dependency(nd_i, lst, c)?;
                els.to_dependency(nd_i, lst, c)
            }
            TExp::Term(t) => t.to_dependency(nd_i, lst, c),
            TExp::Block(block) => block.to_dependency(nd_i, lst, c),
        }
    }
}
impl TBlock {
    pub(super) fn to_dependency(
        &self,
        nd_i: usize,
        lst: &mut HashSet<usize>,
        c: &mut Compiler,
    ) -> CResult<()> {
        let TBlock { stmt, exp } = self;
        for TStmt { id, val } in stmt {
            c.symbol_table
                .push((Some(id.clone()), val.get_type().is_obj_type()));
            val.to_dependency(nd_i, lst, c)?;
        }
        let ret = exp.to_dependency(nd_i, lst, c);
        for _ in stmt {
            c.symbol_table.pop();
        }
        ret
    }
}
impl TTerm {
    pub(super) fn to_dependency(
        &self,
        nd_i: usize,
        lst: &mut HashSet<usize>,
        c: &mut Compiler,
    ) -> CResult<()> {
        match self {
            TLogical::And(a, b) | TLogical::Or(a, b) => {
                a.to_dependency(nd_i, lst, c)?;
                b.to_dependency(nd_i, lst, c)
            }
            TLogical::BitWise(b) => b.to_dependency(nd_i, lst, c),
        }
    }
}
impl TBitWise {
    pub(super) fn to_dependency(
        &self,
        nd_i: usize,
        lst: &mut HashSet<usize>,
        c: &mut Compiler,
    ) -> CResult<()> {
        match self {
            TBitWise::And(a, b) | TBitWise::Or(a, b) | TBitWise::Xor(a, b) => {
                a.to_dependency(nd_i, lst, c)?;
                b.to_dependency(nd_i, lst, c)
            }
            TBitWise::Comp(a) => a.to_dependency(nd_i, lst, c),
        }
    }
}
impl TComp {
    pub(super) fn to_dependency(
        &self,
        nd_i: usize,
        lst: &mut HashSet<usize>,
        c: &mut Compiler,
    ) -> CResult<()> {
        match self {
            TComp::Eq(a, b) | TComp::Neq(a, b) => {
                a.to_dependency(nd_i, lst, c)?;
                b.to_dependency(nd_i, lst, c)
            }
            TComp::Comp2(a) => a.to_dependency(nd_i, lst, c),
        }
    }
}
impl TComp2 {
    pub(super) fn to_dependency(
        &self,
        nd_i: usize,
        lst: &mut HashSet<usize>,
        c: &mut Compiler,
    ) -> CResult<()> {
        match self {
            TComp2::Leq(a, b) | TComp2::Ls(a, b) | TComp2::Geq(a, b) | TComp2::Gt(a, b) => {
                a.to_dependency(nd_i, lst, c)?;
                b.to_dependency(nd_i, lst, c)
            }
            TComp2::Shift(s) => s.to_dependency(nd_i, lst, c),
        }
    }
}
impl TShift {
    pub(super) fn to_dependency(
        &self,
        nd_i: usize,
        lst: &mut HashSet<usize>,
        c: &mut Compiler,
    ) -> CResult<()> {
        match self {
            TShift::Left(a, b) | TShift::Right(a, b) => {
                a.to_dependency(nd_i, lst, c)?;
                b.to_dependency(nd_i, lst, c)
            }
            TShift::Add(a) => a.to_dependency(nd_i, lst, c),
        }
    }
}
impl TAdd {
    pub(super) fn to_dependency(
        &self,
        nd_i: usize,
        lst: &mut HashSet<usize>,
        c: &mut Compiler,
    ) -> CResult<()> {
        match self {
            TAdd::Plus(a, b) | TAdd::Minus(a, b) => {
                a.to_dependency(nd_i, lst, c)?;
                b.to_dependency(nd_i, lst, c)
            }
            TAdd::Factor(f) => f.to_dependency(nd_i, lst, c),
        }
    }
}
impl TFactor {
    pub(super) fn to_dependency(
        &self,
        nd_i: usize,
        lst: &mut HashSet<usize>,
        c: &mut Compiler,
    ) -> CResult<()> {
        match self {
            TFactor::Mul(f, u) | TFactor::Div(f, u) | TFactor::Mod(f, u) => {
                f.to_dependency(nd_i, lst, c)?;
                u.to_dependency(nd_i, lst, c)
            }
            TFactor::Unary(u) => u.to_dependency(nd_i, lst, c),
        }
    }
}
impl TUnary {
    pub(super) fn to_dependency(
        &self,
        nd_i: usize,
        lst: &mut HashSet<usize>,
        c: &mut Compiler,
    ) -> CResult<()> {
        match self {
            TUnary::Not(p) | TUnary::Minus(p) | TUnary::Primary(p) => p.to_dependency(nd_i, lst, c),
        }
    }
}
impl TPrimary {
    pub(super) fn to_dependency(
        &self,
        nd_i: usize,
        lst: &mut HashSet<usize>,
        c: &mut Compiler,
    ) -> CResult<()> {
        match self {
            TPrimary::Int(_) | TPrimary::Bool(_) => Ok(()),
            TPrimary::Exp(e) => e.to_dependency(nd_i, lst, c),
            TPrimary::Id(id, _) => {
                for (id2, _) in &c.symbol_table {
                    match id2 {
                        Some(id2) if id == id2 => return Ok(()),
                        _ => continue,
                    }
                }
                match c.node_offset(id) {
                    Some(u) if u == nd_i => Err(CompileErr::CircularRef),
                    Some(u) => {
                        lst.insert(u);
                        Ok(())
                    }
                    None => Ok(()),
                }
            }
            TPrimary::Last(id, _) => {
                let i = c.node_offset(id).ok_or(CompileErr::InvalidAtLast)?;
                if i == nd_i {
                    return Ok(());
                }
                c.node_info[nd_i].atlast.insert(i);
                c.atlast_manager.add_atlast_refcnt(i);
                Ok(())
            }
            TPrimary::FnCall(_, _, exps) => {
                for exp in exps {
                    exp.to_dependency(nd_i, lst, c)?;
                }
                Ok(())
            }
            TPrimary::Variant(_, _, texps) => {
                for texp in texps {
                    texp.to_dependency(nd_i, lst, c)?;
                }
                Ok(())
            }
            TPrimary::Tuple(texps, _) => {
                for texp in texps {
                    texp.to_dependency(nd_i, lst, c)?;
                }
                Ok(())
            }
        }
    }
}
