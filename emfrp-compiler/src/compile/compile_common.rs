use crate::ast::*;
use crate::insn::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;

use super::dependency::AtLastManager;
#[derive(Clone)]
pub struct Compiler {
    pub(super) codes: Vec<Insn>,
    pub(super) node_info: Vec<NodeInfo>,
    pub(super) symbol_table: Vec<(Option<Id>, IsObjType)>,
    pub(super) types: HashMap<TypeName, Type>,
    pub(super) func_info: Vec<FuncInfo>,
    pub(super) data_info: Vec<DataInfo>,
    pub(super) atlast_manager: AtLastManager,
    pub(super) local_len: usize,
}
pub type IsObjType = bool;
#[derive(Clone)]
pub enum CompiledCode {
    Eval(Type, Vec<Insn>),
    Def(BcDefVar),
}
#[derive(Clone)]
pub struct BcDefVar {
    pub n_new_nodes: usize,
    pub n_new_func: usize,
    pub n_new_data: usize,
    pub n_last: usize,
    pub init: Vec<Insn>,
    pub node: Vec<(usize, Vec<Insn>)>,
    pub func: Vec<(usize, Vec<Insn>)>,
    pub update: Vec<Insn>,
}
#[derive(Debug, Clone)]
pub(super) struct DataInfo {
    pub name: Id,
    pub typ: Type,
    pub is_new: bool,
}
#[derive(Debug, Clone)]
pub(super) struct FuncInfo {
    pub name: Id,
    pub prms: Vec<(Id, Type)>,
    pub ret: Type,
    pub is_new: bool,
}

#[derive(Debug, Clone)]
pub(super) struct NodeInfo {
    pub name: Id,
    pub typ: Type,
    pub prev: HashSet<usize>, // nodes that must be updated before this node
    // nodes that this node points to
    // node b = a@last  => nodeinfo of b contains a
    pub atlast: HashSet<usize>, 
    pub is_new: bool,
    pub has_value: bool,
    pub output_offset: Option<u8>,
    pub input_kind: NodeInputKind,
}

pub enum CompileErr {
    IdNotFound(Id),
    CircularRef,
    InvalidTypeName(TypeName),
    VariantNotFound(VariantName),
    TooManyLocalVars,
    TooManyFields,
    TypeErr(TypeErr),
    InvalidAtLast,
    TypeAlreadyExists,
    ConflictNodeType(String,Vec<String>),
    OverwriteDevInput
}
impl Debug for CompileErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IdNotFound(arg0) => write!(f, "Identifier {} not found.", arg0.0),
            Self::CircularRef => write!(f, "Circular reference detected."),
            Self::InvalidTypeName(arg0) => write!(f, "Type {:?} not found.", arg0),
            Self::VariantNotFound(arg0) => write!(f, "Variant {} not found.", arg0.0),
            Self::TooManyLocalVars => write!(f, "Too many local variables."),
            Self::TooManyFields => write!(f, "Data type with more than 8 fields is prohibited."),
            Self::TypeErr(arg0) => write!(f, "{:?}", arg0),
            Self::InvalidAtLast => write!(
                f,
                "@last operator can only be used after node with initial value"
            ),
            Self::TypeAlreadyExists => write!(f, "Re-definition of type is prohibited"),
            Self::ConflictNodeType(s,ss) => 
            write!(f, "In order to overwrite node {}, with different type, node {:?} also needs re-defining",s,ss),
            Self::OverwriteDevInput => write!(f, "Cannot overwrite input node"),
        }
    }
}
#[derive(Clone)]
pub enum TypeErr {
    Mismatch(Type, Type),
    IncorrectVarN(usize, usize),
    TypeNotFound(TypeName),
    VarNotFound(VariantName),
    InvalidFuncType(Id),
    IdNotFound(Id),
}
impl Debug for TypeErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mismatch(arg0, arg1) => {
                write!(f, "expected {:?} but {:?} were given", arg0, arg1)
            }
            Self::IncorrectVarN(arg0, arg1) => {
                write!(f, "expected {arg0} args but {arg1} were given")
            }
            Self::TypeNotFound(arg0) => write!(f, "type {:?} not found", arg0),
            Self::VarNotFound(arg0) => write!(f, "variant {:?} not founc", arg0.0),
            Self::InvalidFuncType(arg0) => write!(
                f,
                "{:?} is a func type, which cannot be used in this context",
                arg0.0
            ),
            Self::IdNotFound(arg0) => write!(f, "variable {:?} not founc", arg0.0),
        }
    }
}
#[derive(Eq, PartialEq, Debug, Clone)]
pub(super) enum NodeInputKind {
    None,
    Dev,
    User,
}
pub(super) type CResult<T> = Result<T, CompileErr>;
#[derive(Eq, PartialEq, Debug)]
pub(super) enum VarType<'a> {
    Prim(&'a Type),
    Func(Vec<&'a Type>, &'a Type),
}

#[derive(Debug, Clone)]
pub enum Type {
    Int,
    Bool,
    User(String, Vec<(VariantName, Vec<Type>)>),
    Tuple(Vec<Type>),
}
impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::User(l0, _), Self::User(r0, _)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
impl Eq for Type {}

impl Type {
    pub fn is_obj_type(&self) -> bool {
        matches!(self, Type::User(_, _) | Type::Tuple(_))
    }
}
impl Compiler {
    pub fn add_input_node(&mut self, name: &'static str, typ: Type) {
        self.node_info.push(NodeInfo {
            name: Id(name.to_string()),
            typ,
            prev: HashSet::new(),
            atlast: HashSet::new(),
            is_new: false,
            output_offset: None,
            has_value: true,
            input_kind: NodeInputKind::Dev,
        })
    }

    pub fn add_output_node(&mut self, name: &'static str, typ: Type) {
        self.node_info.push(NodeInfo {
            name: Id(name.to_string()),
            typ,
            prev: HashSet::new(),
            atlast: HashSet::new(),
            is_new: false,
            has_value: true,
            output_offset: Some(0),
            input_kind: NodeInputKind::None,
        })
    }

    pub fn new() -> Self {
        let node_info = vec![];
        Self {
            codes: vec![],
            node_info,
            symbol_table: vec![],
            types: HashMap::new(),
            func_info: vec![],
            data_info: vec![],
            atlast_manager: AtLastManager::new(),
            local_len: 0,
        }
    }

    pub(super) fn get_type_from_variant<'a>(
        &'a self,
        name: &VariantName,
    ) -> CResult<(&'a Type, usize, &'a Vec<Type>)> {
        for (_, t) in &self.types {
            match t {
                Type::User(_, vars) => {
                    for (i, &(ref vname, ref elems)) in vars.iter().enumerate() {
                        if vname == name {
                            return Ok((t, i + 1, elems));
                        }
                    }
                }
                _ => (),
            }
        }
        Err(CompileErr::VariantNotFound(name.clone()))
    }

    pub(super) fn get_type_with_type_name<'a>(&'a self, name: &TypeName) -> CResult<Type> {
        match name {
            TypeName::Tuple(typs) => {
                if typs.len() > u8::MAX as usize {
                    return Err(CompileErr::TooManyFields);
                }
                let mut ret = vec![];
                for typ in typs {
                    ret.push(self.get_type_with_type_name(typ)?)
                }
                Ok(Type::Tuple(ret))
            }
            TypeName::User(_) => self
                .types
                .get(name)
                .map(|c| c.clone())
                .ok_or(CompileErr::TypeErr(TypeErr::TypeNotFound(name.clone()))),
            TypeName::Bool => Ok(Type::Bool),
            TypeName::Int => Ok(Type::Int),
        }
    }

    pub(super) fn get_type_with_var_name<'a>(&'a self, id: &Id) -> CResult<VarType<'a>> {
        for NodeInfo {
            name,
            typ,
            prev: _,
            is_new: _,
            atlast: _,
            has_value:_,
            output_offset: _,
            input_kind: _,
        } in &self.node_info
        {
            if name == id {
                return Ok(VarType::Prim(typ));
            }
        }
        for DataInfo {
            name,
            typ,
            is_new: _,
        } in &self.data_info
        {
            if name == id {
                return Ok(VarType::Prim(typ));
            }
        }
        for FuncInfo {
            name,
            prms,
            ret,
            is_new: _,
        } in &self.func_info
        {
            if name == id {
                let prms = prms.iter().map(|(_, t)| t).collect();
                return Ok(VarType::Func(prms, ret));
            }
        }
        Err(CompileErr::IdNotFound(id.clone()))
    }
    pub(super) fn data_offset(&self, id: &Id) -> Option<usize> {
        for (i, e) in self.data_info.iter().enumerate() {
            if id == &e.name {
                return Some(i);
            }
        }
        None
    }
    pub(super) fn func_offset(&self, name: &Id) -> Option<usize> {
        for (i, e) in self.func_info.iter().enumerate() {
            if name == &e.name {
                return Some(i);
            }
        }
        None
    }
    pub fn node_name(&self, offset: usize) -> &Id {
        &self.node_info[offset].name
    }
    pub(super) fn node_offset(&self, id: &Id) -> Option<usize> {
        for (i, e) in self.node_info.iter().enumerate() {
            if id == &e.name {
                return Some(i);
            }
        }
        None
    }
}
pub(super) fn max_entry(t: &Type) -> usize {
    match t {
        Type::Int => 1,
        Type::Bool => 1,
        Type::User(_, vars) => {
            let mut max = 0;
            for &(_, ref types) in vars {
                if max < types.len() {
                    max = types.len();
                }
            }
            assert!(max <= u8::MAX as usize);
            max
        }
        Type::Tuple(v) => v.len(),
    }
}
impl Debug for CompiledCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let def = match self {
            CompiledCode::Eval(_, e) => {
                writeln!(f, "exp:")?;
                writeln!(f, "{:?}", e)?;
                return Ok(());
            }
            CompiledCode::Def(def) => def,
        };
        let BcDefVar {
            n_new_nodes,
            n_new_func,
            n_new_data,
            n_last,
            node,
            func,
            init,
            update,
        } = def;
        writeln!(f, "new node : {}", n_new_nodes)?;
        writeln!(f, "new func : {}", n_new_func)?;
        writeln!(f, "new data : {}", n_new_data)?;
        writeln!(f, "num_last : {}", n_last)?;
        writeln!(f, "node def")?;
        for (i, insn) in node {
            writeln!(f, "  {} {:?}", i, insn)?;
        }
        writeln!(f, "func def")?;
        for (i, insn) in func {
            writeln!(f, "  {i} {:?}", insn)?;
        }
        writeln!(f, "update : ")?;
        for i in update {
            writeln!(f, " {:?}", i)?;
        }
        writeln!(f, "")?;
        writeln!(f, "init:")?;
        for insn in init {
            writeln!(f, "  {:?}", insn)?;
        }
        Ok(())
    }
}
