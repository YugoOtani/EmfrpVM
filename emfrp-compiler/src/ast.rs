#[derive(Debug, Clone)]
pub enum Def {
    Type(TypeDef),
    Var(VarDef),
}
#[derive(Debug, Clone)]
pub enum Program {
    Def(Vec<Def>),
    Exp(Exp),
}
#[derive(Debug, Clone)]
pub enum VarDef {
    Node {
        name: Id,
        typ: TypeName,
        init: Option<Exp>,
        val: Exp,
    },
    Data {
        name: Id,
        typ: TypeName,
        val: Exp,
    },
    Func {
        name: Id,
        ret: TypeName,
        params: Vec<(Id, TypeName)>,
        body: Exp,
    },
}
#[derive(Debug, Clone)]
pub struct TypeDef {
    pub name: TypeName,
    pub variants: Vec<Variant>,
}
#[derive(Debug, Clone)]
pub struct Variant {
    pub constructor: VariantName,
    pub elems: Vec<TypeName>,
}
#[derive(Debug, Clone)]
pub enum Exp {
    Match(Box<Term>, Vec<Branch>),
    If {
        cond: Box<Term>,
        then: Box<Exp>,
        els: Box<Exp>,
    },
    Term(Term),
    Block(Block),
}
#[derive(Debug, Clone)]
pub struct Block {
    pub stmt: Vec<Stmt>,
    pub exp: Box<Exp>,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub id: Id,
    pub tname: TypeName,
    pub val: Exp,
}

#[derive(Debug, Clone)]
pub struct Branch {
    pub pat: Pattern,
    pub exp: Block,
}
#[derive(Debug, Clone)]
pub enum Pattern {
    Int(i32),
    Id(Id),
    Variant(VariantName, Vec<Pattern>),
    Tuple(Vec<Pattern>),
    Bool(bool),
    None,
}
pub type Term = Logical;
#[derive(Debug, Clone)]
pub enum Logical {
    And(Box<Logical>, BitWise),
    Or(Box<Logical>, BitWise),
    BitWise(BitWise),
}
#[derive(Debug, Clone)]
pub enum BitWise {
    And(Box<BitWise>, Comp),
    Or(Box<BitWise>, Comp),
    Xor(Box<BitWise>, Comp),
    Comp(Comp),
}
#[derive(Debug, Clone)]
pub enum Comp {
    Eq(Box<Comp>, Comp2),
    Neq(Box<Comp>, Comp2),
    Comp2(Comp2),
}
#[derive(Debug, Clone)]
pub enum Comp2 {
    Leq(Box<Comp2>, Shift),
    Ls(Box<Comp2>, Shift),
    Geq(Box<Comp2>, Shift),
    Gt(Box<Comp2>, Shift),
    Shift(Box<Shift>),
}
#[derive(Debug, Clone)]
pub enum Shift {
    Left(Box<Shift>, Add),
    Right(Box<Shift>, Add),
    Add(Add),
}
#[derive(Debug, Clone)]
pub enum Add {
    Plus(Box<Add>, Factor),
    Minus(Box<Add>, Factor),
    Factor(Factor),
}
#[derive(Debug, Clone)]
pub enum Factor {
    Mul(Box<Factor>, Unary),
    Div(Box<Factor>, Unary),
    Mod(Box<Factor>, Unary),
    Unary(Unary),
}
#[derive(Debug, Clone)]
pub enum Unary {
    Not(Primary),
    Minus(Primary),
    Primary(Primary),
}
#[derive(Debug, Clone)]
pub enum Primary {
    Int(i32),
    Bool(bool),
    Exp(Box<Exp>),
    Id(Id),
    Last(Id),
    Variant(VariantName, Vec<Exp>),
    Tuple(Vec<Exp>),
    FnCall(Id, Vec<Exp>),
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Id(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeName {
    Tuple(Vec<TypeName>),
    User(String),
    Bool,
    Int,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantName(pub String);
