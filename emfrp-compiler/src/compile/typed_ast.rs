use super::compile_common::*;
use crate::ast::*;
//Typed Ast
#[derive(Debug, Clone)]
pub(super) enum TVarDef {
    Node {
        name: Id,
        init: Option<TExp>,
        val: TExp,
    },
    Data {
        name: Id,
        val: TExp,
    },
    Func {
        name: Id,
        params: Vec<(Id, Type)>,
        body: TExp,
    },
}

#[derive(Debug, Clone)]
pub(super) enum TExp {
    Match(Box<TTerm>, Vec<TBranch>),
    If {
        cond: Box<TTerm>,
        then: Box<TExp>,
        els: Box<TExp>,
    },
    Term(TTerm),
    Block(TBlock),
}
#[derive(Debug, Clone)]
pub(super) struct TBlock {
    pub stmt: Vec<TStmt>,
    pub exp: Box<TExp>,
}
#[derive(Debug, Clone)]
pub(super) struct TStmt {
    pub id: Id,
    pub val: TExp,
}

#[derive(Debug, Clone)]
pub(super) struct TBranch {
    pub pat: TPattern,
    pub exp: TBlock,
}
#[derive(Debug, Clone)]
pub(super) enum TPattern {
    Int(i32),
    Id(Type, Id),
    Variant(usize, Vec<TPattern>),
    Bool(bool),
    Tuple(Vec<TPattern>),
    None,
}
pub(super) type TTerm = TLogical;
#[derive(Debug, Clone)]
pub(super) enum TLogical {
    And(Box<TLogical>, TBitWise),
    Or(Box<TLogical>, TBitWise),
    BitWise(TBitWise),
}
#[derive(Debug, Clone)]
pub(super) enum TBitWise {
    And(Box<TBitWise>, TComp),
    Or(Box<TBitWise>, TComp),
    Xor(Box<TBitWise>, TComp),
    Comp(TComp),
}
#[derive(Debug, Clone)]
pub(super) enum TComp {
    Eq(Box<TComp>, TComp2),
    Neq(Box<TComp>, TComp2),
    Comp2(TComp2),
}
#[derive(Debug, Clone)]
pub(super) enum TComp2 {
    Leq(Box<TComp2>, TShift),
    Ls(Box<TComp2>, TShift),
    Geq(Box<TComp2>, TShift),
    Gt(Box<TComp2>, TShift),
    Shift(Box<TShift>),
}
#[derive(Debug, Clone)]
pub(super) enum TShift {
    Left(Box<TShift>, TAdd),
    Right(Box<TShift>, TAdd),
    Add(TAdd),
}
#[derive(Debug, Clone)]
pub(super) enum TAdd {
    Plus(Box<TAdd>, TFactor),
    Minus(Box<TAdd>, TFactor),
    Factor(TFactor),
}
#[derive(Debug, Clone)]
pub(super) enum TFactor {
    Mul(Box<TFactor>, TUnary),
    Div(Box<TFactor>, TUnary),
    Mod(Box<TFactor>, TUnary),
    Unary(TUnary),
}
#[derive(Debug, Clone)]
pub(super) enum TUnary {
    Not(TPrimary),
    Minus(TPrimary),
    Primary(TPrimary),
}
#[derive(Debug, Clone)]
pub(super) enum TPrimary {
    Int(i32),
    Bool(bool),
    Exp(Box<TExp>),
    Id(Id, Type),
    Last(Id, Type),
    Variant(VariantName, Type, Vec<TExp>),
    FnCall(Id, Type, Vec<TExp>),
    Tuple(Vec<TExp>, Type),
}
impl TBlock {
    pub(super) fn get_type(&self) -> &Type {
        self.exp.get_type()
    }
}
impl TExp {
    pub(super) fn get_type(&self) -> &Type {
        match self {
            TExp::Match(_, branch) => branch[0].exp.get_type(),
            TExp::If {
                cond: _,
                then: _,
                els,
            } => els.get_type(),
            TExp::Term(t) => t.get_type(),
            TExp::Block(TBlock { stmt: _, exp }) => exp.get_type(),
        }
    }
}
impl TTerm {
    pub(super) fn get_type(&self) -> &Type {
        match self {
            TLogical::And(_, _) => &Type::Bool,
            TLogical::Or(_, _) => &Type::Bool,
            TLogical::BitWise(b) => b.get_type(),
        }
    }
}
impl TBitWise {
    pub(super) fn get_type(&self) -> &Type {
        match self {
            TBitWise::And(_, _) => &Type::Int,
            TBitWise::Or(_, _) => &Type::Int,
            TBitWise::Xor(_, _) => &Type::Int,
            TBitWise::Comp(c) => c.get_type(),
        }
    }
}
impl TComp {
    pub(super) fn get_type(&self) -> &Type {
        match self {
            TComp::Eq(_, _) => &Type::Bool,
            TComp::Neq(_, _) => &Type::Bool,
            TComp::Comp2(c) => c.get_type(),
        }
    }
}
impl TComp2 {
    pub(super) fn get_type(&self) -> &Type {
        match self {
            TComp2::Leq(_, _) => &Type::Bool,
            TComp2::Ls(_, _) => &Type::Bool,
            TComp2::Geq(_, _) => &Type::Bool,
            TComp2::Gt(_, _) => &Type::Bool,
            TComp2::Shift(s) => s.get_type(),
        }
    }
}
impl TShift {
    pub(super) fn get_type(&self) -> &Type {
        match self {
            TShift::Left(_, _) => &Type::Int,
            TShift::Right(_, _) => &Type::Int,
            TShift::Add(a) => a.get_type(),
        }
    }
}
impl TAdd {
    pub(super) fn get_type(&self) -> &Type {
        match self {
            TAdd::Plus(_, _) => &Type::Int,
            TAdd::Minus(_, _) => &Type::Int,
            TAdd::Factor(f) => f.get_type(),
        }
    }
}
impl TFactor {
    pub(super) fn get_type(&self) -> &Type {
        match self {
            TFactor::Mul(_, _) => &Type::Int,
            TFactor::Div(_, _) => &Type::Int,
            TFactor::Mod(_, _) => &Type::Int,
            TFactor::Unary(u) => u.get_type(),
        }
    }
}
impl TUnary {
    pub(super) fn get_type(&self) -> &Type {
        match self {
            TUnary::Not(_) => &Type::Bool,
            TUnary::Minus(_) => &Type::Int,
            TUnary::Primary(p) => p.get_type(),
        }
    }
}
impl TPrimary {
    pub(super) fn get_type(&self) -> &Type {
        match self {
            TPrimary::Int(_) => &Type::Int,
            TPrimary::Bool(_) => &Type::Bool,
            TPrimary::Exp(e) => e.get_type(),
            TPrimary::Id(_, t) => t,
            TPrimary::Last(_, t) => t,
            TPrimary::FnCall(_, t, _) => t,
            TPrimary::Variant(_, t, _) => t,
            TPrimary::Tuple(_, t) => t,
        }
    }
}
