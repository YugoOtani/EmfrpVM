use super::compile_common::*;
use super::typed_ast::*;
use crate::ast::*;

impl Compiler {
    pub(super) fn exp_type_check(&self, e: Exp) -> CResult<TExp> {
        let mut locals = vec![];
        let ret = e.typed(self, &mut locals)?;
        assert_eq!(locals.len(), 0);
        return Ok(ret);
    }
    pub(super) fn vardef_type_check(&self, d: VarDef) -> CResult<TVarDef> {
        let mut locals = vec![];
        let ret = d.typed(self, &mut locals)?;
        assert_eq!(locals.len(), 0);
        return Ok(ret);
    }
}

fn terr<T>(t: TypeErr) -> CResult<T> {
    Err(CompileErr::TypeErr(t))
}

impl VarDef {
    fn typed(self, c: &Compiler, locals: &mut Vec<(Id, Type)>) -> CResult<TVarDef> {
        match self {
            VarDef::Node {
                name,
                typ,
                init,
                val,
            } => {
                if let Some(init) = init {
                    let typ = c.get_type_with_type_name(&typ)?;
                    let tinit = init.typed(c, locals)?;
                    let val = val.typed(c, locals)?;
                    if &typ != tinit.get_type() {
                        return terr(TypeErr::Mismatch(typ, tinit.get_type().clone()));
                    }
                    if &typ != val.get_type() {
                        return terr(TypeErr::Mismatch(typ, val.get_type().clone()));
                    }
                    Ok(TVarDef::Node {
                        name,
                        init: Some(tinit),
                        val,
                    })
                } else {
                    let typ = c.get_type_with_type_name(&typ)?;
                    let val = val.typed(c, locals)?;
                    if &typ == val.get_type() {
                        Ok(TVarDef::Node {
                            name,
                            init: None,
                            val,
                        })
                    } else {
                        terr(TypeErr::Mismatch(typ, val.get_type().clone()))
                    }
                }
            }
            VarDef::Data { name, typ, val } => {
                let typ = c.get_type_with_type_name(&typ)?;
                let val = val.typed(c, locals)?;
                if &typ == val.get_type() {
                    Ok(TVarDef::Data { name, val })
                } else {
                    terr(TypeErr::Mismatch(typ, val.get_type().clone()))
                }
            }

            VarDef::Func {
                name,
                ret,
                params,
                body,
            } => {
                let mut params_ast = Vec::with_capacity(params.len());
                let mut prms = Vec::with_capacity(params.len());
                for (id, typ) in params {
                    let typ = c.get_type_with_type_name(&typ)?;
                    locals.push((id.clone(), typ.clone()));
                    prms.push(typ.clone());
                    params_ast.push((id, typ.clone()))
                }
                let ret = c.get_type_with_type_name(&ret)?;
                let body = body.typed(c, locals)?;
                if body.get_type() == &ret {
                    for _ in 0..params_ast.len() {
                        locals.pop();
                    }
                    Ok(TVarDef::Func {
                        name,
                        params: params_ast,
                        body,
                    })
                } else {
                    terr(TypeErr::Mismatch(ret, body.get_type().clone()))
                }
            }
        }
    }
}
impl Exp {
    fn typed(self, c: &Compiler, locals: &mut Vec<(Id, Type)>) -> CResult<TExp> {
        match self {
            Exp::Match(e, branches) => {
                let e = e.typed(c, locals)?;
                let etype = e.get_type();
                let mut rettype = None;
                let mut branch_ret = Vec::with_capacity(branches.len());
                for Branch { pat, exp } in branches {
                    // check whether pat is the same type as etype
                    let l0 = locals.len();
                    let pat = pat.typed(c, locals, &etype)?;
                    let l1 = locals.len();
                    let exp = exp.typed(c, locals)?;
                    assert_eq!(locals.len(), l1);
                    for _ in 0..l1 - l0 {
                        locals.pop();
                    }
                    // check whether all exp is the same
                    match rettype {
                        None => rettype = Some(exp.get_type().clone()),
                        Some(ret) if &ret != exp.get_type() => {
                            return terr(TypeErr::Mismatch(ret, exp.get_type().clone()))
                        }
                        _ => (),
                    }
                    branch_ret.push(TBranch { pat, exp });
                }
                Ok(TExp::Match(Box::new(e), branch_ret))
            }
            Exp::If { cond, then, els } => {
                let cond = cond.typed(c, locals)?;
                let then = then.typed(c, locals)?;
                let els = els.typed(c, locals)?;

                if (&Type::Bool) != cond.get_type() {
                    return terr(TypeErr::Mismatch(Type::Bool, cond.get_type().clone()));
                }
                if then.get_type() != els.get_type() {
                    return terr(TypeErr::Mismatch(
                        then.get_type().clone(),
                        els.get_type().clone(),
                    ));
                }
                Ok(TExp::If {
                    cond: Box::new(cond),
                    then: Box::new(then),
                    els: Box::new(els),
                })
            }
            Exp::Block(block) => Ok(TExp::Block(block.typed(c, locals)?)),
            Exp::Term(t) => Ok(TExp::Term(t.typed(c, locals)?)),
        }
    }
}
impl Block {
    fn typed(self, c: &Compiler, locals: &mut Vec<(Id, Type)>) -> CResult<TBlock> {
        let i0 = locals.len();
        let Block { stmt, exp } = self;
        let mut tstmt = Vec::with_capacity(stmt.len());
        for Stmt { id, tname, val } in stmt {
            let t = c.get_type_with_type_name(&tname)?;
            let tval = val.typed(c, locals)?;
            if tval.get_type() != &t {
                return terr(TypeErr::Mismatch(tval.get_type().clone(), t));
            }
            locals.push((id.clone(), t.clone()));
            tstmt.push(TStmt { id, val: tval })
        }
        let texp = exp.typed(c, locals)?;
        for _ in 0..tstmt.len() {
            locals.pop();
        }
        assert_eq!(locals.len(), i0);
        Ok(TBlock {
            stmt: tstmt,
            exp: Box::new(texp),
        })
    }
}
impl Pattern {
    fn typed(
        self,
        c: &Compiler,
        locals: &mut Vec<(Id, Type)>,
        match_type: &Type,
    ) -> CResult<TPattern> {
        match self {
            Pattern::Int(i) => match &match_type {
                Type::Int => Ok(TPattern::Int(i)),
                _ => terr(TypeErr::Mismatch(Type::Int, match_type.clone())),
            },
            Pattern::Id(id) => {
                locals.push((id.clone(), match_type.clone()));
                Ok(TPattern::Id(match_type.clone(), id))
            }
            Pattern::Variant(vname, pat) => {
                let (tinfo, tag, prms) = c.get_type_from_variant(&vname)?;
                if tinfo != match_type {
                    return terr(TypeErr::Mismatch(tinfo.clone(), match_type.clone()));
                }
                if prms.len() != pat.len() {
                    return terr(TypeErr::IncorrectVarN(prms.len(), pat.len()));
                }
                let mut tpats = Vec::with_capacity(pat.len());
                for (typ, pat) in prms.iter().zip(pat.into_iter()) {
                    tpats.push(pat.typed(c, locals, typ)?);
                }

                Ok(TPattern::Variant(tag, tpats))
            }
            Pattern::Bool(b) => match match_type {
                Type::Bool => Ok(TPattern::Bool(b)),
                _ => terr(TypeErr::Mismatch(Type::Bool, match_type.clone())),
            },
            Pattern::Tuple(pats) => {
                let types = match match_type {
                    Type::Tuple(ref inside) => inside,
                    t => return terr(TypeErr::Mismatch(Type::Tuple(vec![]), t.clone())),
                };
                if types.len() != pats.len() {
                    return terr(TypeErr::IncorrectVarN(types.len(), pats.len()));
                }
                let len = types.len();
                let mut tpats = Vec::with_capacity(len);
                for (pat, t) in pats.into_iter().zip(types.iter()) {
                    tpats.push(pat.typed(c, locals, t)?);
                }
                Ok(TPattern::Tuple(tpats))
            }
            Pattern::None => Ok(TPattern::None),
        }
    }
}
impl Term {
    fn typed(self, c: &Compiler, locals: &mut Vec<(Id, Type)>) -> CResult<TTerm> {
        match self {
            Logical::And(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Bool) {
                    return terr(TypeErr::Mismatch(Type::Bool, a.get_type().clone()));
                }
                Ok(TLogical::And(Box::new(a), b))
            }
            Logical::Or(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Bool) {
                    return terr(TypeErr::Mismatch(Type::Bool, a.get_type().clone()));
                }
                Ok(TLogical::Or(Box::new(a), b))
            }
            Logical::BitWise(b) => Ok(TLogical::BitWise(b.typed(c, locals)?)),
        }
    }
}
impl BitWise {
    fn typed(self, c: &Compiler, locals: &mut Vec<(Id, Type)>) -> CResult<TBitWise> {
        match self {
            BitWise::And(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TBitWise::And(Box::new(a), b))
            }
            BitWise::Or(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TBitWise::Or(Box::new(a), b))
            }
            BitWise::Xor(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TBitWise::Xor(Box::new(a), b))
            }
            BitWise::Comp(cmp) => Ok(TBitWise::Comp(cmp.typed(c, locals)?)),
        }
    }
}
impl Comp {
    fn typed(self, c: &Compiler, locals: &mut Vec<(Id, Type)>) -> CResult<TComp> {
        match self {
            Comp::Eq(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TComp::Eq(Box::new(a), b))
            }
            Comp::Neq(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TComp::Neq(Box::new(a), b))
            }
            Comp::Comp2(cmp2) => Ok(TComp::Comp2(cmp2.typed(c, locals)?)),
        }
    }
}
impl Comp2 {
    fn typed(self, c: &Compiler, locals: &mut Vec<(Id, Type)>) -> CResult<TComp2> {
        match self {
            Comp2::Leq(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TComp2::Leq(Box::new(a), b))
            }
            Comp2::Ls(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TComp2::Ls(Box::new(a), b))
            }
            Comp2::Geq(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TComp2::Geq(Box::new(a), b))
            }
            Comp2::Gt(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TComp2::Gt(Box::new(a), b))
            }
            Comp2::Shift(sft) => Ok(TComp2::Shift(Box::new(sft.typed(c, locals)?))),
        }
    }
}
impl Shift {
    fn typed(self, c: &Compiler, locals: &mut Vec<(Id, Type)>) -> CResult<TShift> {
        match self {
            Shift::Left(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TShift::Left(Box::new(a), b))
            }
            Shift::Right(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TShift::Right(Box::new(a), b))
            }
            Shift::Add(add) => Ok(TShift::Add(add.typed(c, locals)?)),
        }
    }
}
impl Add {
    fn typed(self, c: &Compiler, locals: &mut Vec<(Id, Type)>) -> CResult<TAdd> {
        match self {
            Add::Plus(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TAdd::Plus(Box::new(a), b))
            }
            Add::Minus(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TAdd::Minus(Box::new(a), b))
            }
            Add::Factor(f) => Ok(TAdd::Factor(f.typed(c, locals)?)),
        }
    }
}
impl Factor {
    fn typed(self, c: &Compiler, locals: &mut Vec<(Id, Type)>) -> CResult<TFactor> {
        match self {
            Factor::Mul(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TFactor::Mul(Box::new(a), b))
            }
            Factor::Div(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TFactor::Div(Box::new(a), b))
            }
            Factor::Mod(a, b) => {
                let a = a.typed(c, locals)?;
                let b = b.typed(c, locals)?;
                if a.get_type() != b.get_type() {
                    return terr(TypeErr::Mismatch(
                        a.get_type().clone(),
                        b.get_type().clone(),
                    ));
                }
                if a.get_type() != (&Type::Int) {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TFactor::Mod(Box::new(a), b))
            }
            Factor::Unary(u) => Ok(TFactor::Unary(u.typed(c, locals)?)),
        }
    }
}
impl Unary {
    fn typed(self, c: &Compiler, locals: &mut Vec<(Id, Type)>) -> CResult<TUnary> {
        match self {
            Unary::Not(prim) => {
                let a = prim.typed(c, locals)?;
                if a.get_type() != &Type::Bool {
                    return terr(TypeErr::Mismatch(Type::Bool, a.get_type().clone()));
                }
                Ok(TUnary::Not(a))
            }
            Unary::Minus(prim) => {
                let a = prim.typed(c, locals)?;
                if a.get_type() != &Type::Int {
                    return terr(TypeErr::Mismatch(Type::Int, a.get_type().clone()));
                }
                Ok(TUnary::Minus(a))
            }
            Unary::Primary(prim) => Ok(TUnary::Primary(prim.typed(c, locals)?)),
        }
    }
}
impl Primary {
    fn typed(self, c: &Compiler, locals: &mut Vec<(Id, Type)>) -> CResult<TPrimary> {
        match self {
            Primary::Int(i) => Ok(TPrimary::Int(i)),
            Primary::Bool(b) => Ok(TPrimary::Bool(b)),
            Primary::Exp(e) => Ok(TPrimary::Exp(Box::new(e.typed(c, locals)?))),
            Primary::Id(id) => {
                for i in (0..locals.len()).rev() {
                    if &locals[i].0 == &id {
                        return Ok(TPrimary::Id(id, locals[i].1.clone()));
                    }
                }
                match c.get_type_with_var_name(&id)? {
                    VarType::Prim(p) => Ok(TPrimary::Id(id, p.clone())),
                    VarType::Func(_, _) => terr(TypeErr::InvalidFuncType(id.clone())),
                }
            }
            Primary::Last(id) => match c.get_type_with_var_name(&id)? {
                VarType::Prim(p) => Ok(TPrimary::Last(id, p.clone())),
                VarType::Func(_, _) => terr(TypeErr::InvalidFuncType(id.clone())),
            },
            Primary::FnCall(id, args) => {
                if let VarType::Func(prms, ret) = c.get_type_with_var_name(&id)? {
                    if prms.len() != args.len() {
                        return terr(TypeErr::IncorrectVarN(prms.len(), args.len()));
                    }
                    let mut targs = Vec::with_capacity(args.len());
                    for (typ_expected, arg) in prms.into_iter().zip(args.into_iter()) {
                        let arg = arg.typed(c, locals)?;
                        if arg.get_type() != typ_expected {
                            return terr(TypeErr::Mismatch(
                                typ_expected.clone(),
                                arg.get_type().clone(),
                            ));
                        }
                        targs.push(arg);
                    }
                    Ok(TPrimary::FnCall(id, ret.clone(), targs))
                } else {
                    terr(TypeErr::InvalidFuncType(id.clone()))
                }
            }
            Primary::Variant(name, exps) => {
                let (ret_t, _, e_t) = c.get_type_from_variant(&name)?;
                if exps.len() != e_t.len() {
                    return terr(TypeErr::IncorrectVarN(exps.len(), e_t.len()));
                }
                let mut texps = Vec::with_capacity(exps.len());
                for (e, t) in exps.into_iter().zip(e_t.iter()) {
                    let texp = e.typed(c, locals)?;
                    if texp.get_type() != t {
                        return terr(TypeErr::Mismatch(t.clone(), texp.get_type().clone()));
                    } else {
                        texps.push(texp)
                    }
                }
                Ok(TPrimary::Variant(name, ret_t.clone(), texps))
            }
            Primary::Tuple(v) => {
                let mut texp = Vec::with_capacity(v.len());
                let mut tv = Vec::with_capacity(v.len());
                for e in v {
                    let e = e.typed(c, locals)?;
                    tv.push(e.get_type().clone());
                    texp.push(e);
                }

                Ok(TPrimary::Tuple(texp, Type::Tuple(tv)))
            }
        }
    }
}
