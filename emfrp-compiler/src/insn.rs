use std::fmt::Debug;

use crate::{
    compile::compile_common::{BcDefVar, CompiledCode},
    DEBUG,
};
#[derive(Clone, Eq, PartialEq)]
pub enum Insn {
    None,
    Nil,
    Not,
    Minus,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    ShiftL,
    ShiftR,
    Ls,
    Leq,
    Gt,
    Geq,
    Eq,
    Neq,
    BitAnd,
    BitOr,
    BitXor,
    Return,
    Print,
    PrintObj,
    Halt,
    Placeholder,
    PushTrue,
    PushFalse,
    Je0,
    Je1,
    Je8(i8),
    Je16(i16),
    Je32(i32),
    J0,
    J1,
    J8(i8),
    J16(i16),
    J32(i32),
    Jne0,
    Jne1,
    Jne8(i8),
    Jne16(i16),
    Jne32(i32),
    Int(SignedNum),                   // 0,1,2,3,4,5,6,i8,i16,i32
    GetData(UnsignedNum),             //u8,u32
    GetLocal(SignedNum),              //0,1,2,3,4,5,6,i8,i16,i32
    SetLocal(SignedNum),              //0,1,2,3,4,5,6,i8,i16,i32
    UpdateNode(UnsignedNum),          //u8,u16,u32
    UpdateDev(UnsignedNum),           //0,1,2,3,u8
    GetNode(UnsignedNum),             //u8,u16,u32
    SetNode(UnsignedNum),             //u8,u16,u32
    SetData(UnsignedNum),             //u8,u16,u32
    EndUpdateNode(UnsignedNum),       //u8,u16,u32
    GetLast(UnsignedNum),             //0,1,2,3,u8,u16,u32
    SetLast(UnsignedNum),             //0,1,2,3,u8,u16,u32
    Call(NArgs, UnsignedNum),         // FuncOffset: u8,u16,u32
    AllocObj(UnsignedNum, ObjHeader), // MaxEntry : 0,1,2,3,4,5,6
    Peek,

    Pop(UnsignedNum), //1,2,3,4,5,6,u8,u16,u32
    ObjTag,
    ObjField(UnsignedNum),     //0,1,2,3,4,5,6
    AllocLocal(UnsignedNum),   //0,1,2,3,4,5,6,u8,u16,u32
    OutputAction(UnsignedNum), //0,1,2,3,u8

    EndUpdateNodeObj(UnsignedNum), //u8,u16,u32
    DropLocalObj(SignedNum),       //0,1,2,3,4,5,6,i8,i16,i32
    GetNodeRef(UnsignedNum),       //u8,u16,u32
    GetDataRef(UnsignedNum),       //u8, u16, u32
    GetLocalRef(SignedNum),        //0,1,2,3,4,5,6,i8,i16,i32
    SetLocalRef(SignedNum),        //0,1,2,3,4,5,6,i8,i16,i32
    ObjFieldRef(UnsignedNum),      //0,1,2,3,4,5,6
    GetLastRef(UnsignedNum),       //0,1,2,3,u8,u16,u32
    SetLastRef(UnsignedNum),       //0,1,2,3,u8,u16,u32
    SetNodeRef(UnsignedNum),       //u8,u16,u32
    SetDataRef(UnsignedNum),       //u8,u16,u32
    DropLast(UnsignedNum),         // u8,u16,u32
    Abort,
}
type NArgs = u8;

#[derive(Hash)]
struct Placeholder {
    p_offset: usize,
    bc_start: Option<usize>,
}
pub struct ToByteCode {
    dbg_info: Vec<(Option<&'static str>, usize, usize)>,
    bytecode: Vec<u8>,
}
impl ToByteCode {
    fn new() -> Self {
        Self {
            dbg_info: vec![],
            bytecode: vec![],
        }
    }
    fn get(self) -> Vec<u8> {
        self.bytecode
    }
    fn dbg_bytecode(&self) {
        for &(section, st, ed) in &self.dbg_info {
            match section {
                Some(s) => println!("{s} : {:?}", &self.bytecode[st..ed]),
                None => println!("{:?}", &self.bytecode[st..ed]),
            }
        }
    }
    fn push_u16(&mut self, i: u16, name: Option<&'static str>) {
        let len = self.bytecode.len();
        push_u16_le(i, &mut self.bytecode);
        self.dbg_info.push((name, len, len + 2));
    }
    fn push_byte_code_len(&mut self, insns: &Vec<Insn>, name: Option<&'static str>) {
        let len = self.bytecode.len();
        self.dbg_info.push((name, len, len + 2));
        push_u16_le(bytecode_len(&insns) as u16, &mut self.bytecode);
    }
    fn push_byte_code(&mut self, insns: &Vec<Insn>, section: Option<&'static str>) {
        let i0 = self.bytecode.len();
        for insn in insns {
            insn.push_byte_code(&mut self.bytecode);
        }
        let i1 = self.bytecode.len();
        self.dbg_info.push((section, i0, i1));
    }
    fn len_placeholder_new(&mut self, name: Option<&'static str>) -> Placeholder {
        let len = self.bytecode.len();
        push_u16_le(0, &mut self.bytecode);
        self.dbg_info.push((name, len, len + 2));
        Placeholder {
            p_offset: len,
            bc_start: None,
        }
    }
    fn len_placeholder_start(&mut self, mut p: Placeholder) -> Placeholder {
        let len = self.bytecode.len();
        p.bc_start = Some(len);
        p
    }
    fn len_placeholder_stop(&mut self, p: Placeholder) {
        let end = self.bytecode.len();
        let Placeholder {
            p_offset: i,
            bc_start,
        } = p;
        let dif = end - bc_start.unwrap();
        for (j, b) in (dif as u16).to_le_bytes().iter().enumerate() {
            self.bytecode[i + j] = *b;
        }
    }
}

pub fn to_byte_code(code: &CompiledCode) -> Vec<u8> {
    let mut bc = ToByteCode::new();
    let datalen_p = bc.len_placeholder_new(Some("datasize"));
    let datalen_p = bc.len_placeholder_start(datalen_p);
    let code = match code {
        CompiledCode::Eval(_, e) => {
            bc.bytecode.push(1);
            bc.push_byte_code(e, Some("exp"));
            bc.len_placeholder_stop(datalen_p);
            return bc.get();
        }
        CompiledCode::Def(code) => code,
    };
    let BcDefVar {
        n_new_nodes,
        n_new_data,
        n_new_func,
        n_last,
        update,
        node,
        func,
        init,
    } = code;

    bc.bytecode.push(0);
    bc.push_byte_code_len(init, Some("init len"));
    bc.push_byte_code_len(update, Some("update"));
    bc.push_u16(*n_last as u16, Some("num last"));
    bc.push_u16(node.len() as u16, Some("node def len"));
    bc.push_u16(func.len() as u16, Some("func def len"));
    bc.push_u16(*n_new_nodes as u16, Some("new node len"));
    bc.push_u16(*n_new_func as u16, Some("new func len"));
    bc.push_u16(*n_new_data as u16, Some("new data len"));

    for &(i, ref insn) in node {
        bc.push_u16(i as u16, Some("node offset"));
        bc.push_byte_code_len(insn, Some("def len"));
        bc.push_byte_code(insn, Some("def"));
    }
    for &(i, ref insn) in func {
        bc.push_u16(i as u16, Some("func offset"));
        bc.push_byte_code_len(insn, Some("def len"));
        bc.push_byte_code(insn, Some("def"));
    }
    bc.push_byte_code(update, Some("update"));

    bc.push_byte_code(init, Some("init"));

    bc.len_placeholder_stop(datalen_p);
    if DEBUG {
        bc.dbg_bytecode();
    }
    bc.get()
}
impl Debug for Insn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "{:>2}:None", self.op_code()),
            Self::Nil => write!(f, "{:>2}:Nil", self.op_code()),
            Self::Not => write!(f, "{:>2}:Not", self.op_code()),
            Self::Minus => write!(f, "{:>2}:Minus", self.op_code()),
            Self::Add => write!(f, "{:>2}:Add", self.op_code()),
            Self::Sub => write!(f, "{:>2}:Sub", self.op_code()),
            Self::Mul => write!(f, "{:>2}:Mul", self.op_code()),
            Self::Div => write!(f, "{:>2}:Div", self.op_code()),
            Self::Mod => write!(f, "{:>2}:Mod", self.op_code()),
            Self::ShiftL => write!(f, "{:>2}:ShiftL", self.op_code()),
            Self::ShiftR => write!(f, "{:>2}:ShiftR", self.op_code()),
            Self::Ls => write!(f, "{:>2}:Ls", self.op_code()),
            Self::Leq => write!(f, "{:>2}:Leq", self.op_code()),
            Self::Gt => write!(f, "{:>2}:Gt", self.op_code()),
            Self::Geq => write!(f, "{:>2}:Geq", self.op_code()),
            Self::Eq => write!(f, "{:>2}:Eq", self.op_code()),
            Self::Neq => write!(f, "{:>2}:Neq", self.op_code()),
            Self::BitAnd => write!(f, "{:>2}:BitAnd", self.op_code()),
            Self::BitOr => write!(f, "{:>2}:BitOr", self.op_code()),
            Self::BitXor => write!(f, "{:>2}:BitXor", self.op_code()),
            Self::Return => write!(f, "{:>2}:Return", self.op_code()),

            Self::Print => write!(f, "{:>2}:Print", self.op_code()),
            Self::PrintObj => write!(f, "{:>2}:PrintObj", self.op_code()),
            Self::Halt => write!(f, "{:>2}:Halt", self.op_code()),
            Self::Placeholder => write!(f, "{:>2}:Placeholder", self.op_code()),
            Self::PushTrue => write!(f, "{:>2}:PushTrue", self.op_code()),
            Self::PushFalse => write!(f, "{:>2}:PushFalse", self.op_code()),
            Self::J0 => write!(f, "{:>2}:J0", self.op_code()),
            Self::J1 => write!(f, "{:>2}:J1", self.op_code()),
            Self::Je0 => write!(f, "{:>2}:Je0", self.op_code()),
            Self::Je1 => write!(f, "{:>2}:Je1", self.op_code()),
            Self::Jne0 => write!(f, "{:>2}:Jne0", self.op_code()),
            Self::Jne1 => write!(f, "{:>2}:Jne1", self.op_code()),

            Self::Je8(arg0) => write!(f, "{:>2}:Je8({})", self.op_code(), arg0),
            Self::J8(arg0) => write!(f, "{:>2}:J8({})", self.op_code(), arg0),
            Self::Jne8(arg0) => write!(f, "{:>2}:Jne8({})", self.op_code(), arg0),
            Self::Int(arg0) => write!(f, "{:>2}:Int({})", self.op_code(), arg0.to_i32()),
            Self::Je16(arg0) => write!(f, "{:>2}:Je16({})", self.op_code(), arg0),
            Self::Jne16(arg0) => write!(f, "{:>2}:Jne16({})", self.op_code(), arg0),
            Self::J16(arg0) => write!(f, "{:>2}:J16({})", self.op_code(), arg0),
            Self::Je32(arg0) => write!(f, "{:>2}:Je32({})", self.op_code(), arg0),
            Self::Jne32(arg0) => write!(f, "{:>2}:Jne32({})", self.op_code(), arg0),
            Self::J32(arg0) => write!(f, "{:>2}:J32({})", self.op_code(), arg0),
            Self::GetData(arg0) => write!(f, "{:>2}:GetData({})", self.op_code(), arg0.to_u32()),
            Self::GetLocal(arg0) => write!(f, "{:>2}:GetLocal({})", self.op_code(), arg0.to_i32()),
            Self::SetLocal(arg0) => write!(f, "{:>2}:SetLocal({})", self.op_code(), arg0.to_i32()),
            Self::UpdateNode(arg0) => {
                write!(f, "{:>2}:UpdateNode({})", self.op_code(), arg0.to_u32())
            }
            Self::GetNode(arg0) => write!(f, "{:>2}:GetNode({})", self.op_code(), arg0.to_u32()),
            Self::SetNode(arg0) => write!(f, "{:>2}:SetNode({})", self.op_code(), arg0.to_u32()),
            Self::EndUpdateNode(arg0) => {
                write!(f, "{:>2}:EndUpdNode({})", self.op_code(), arg0.to_u32())
            }
            Self::GetLast(arg0) => write!(f, "{:>2}:GetLast({})", self.op_code(), arg0.to_u32()),
            Self::SetData(arg0) => write!(f, "{:>2}:SetData({})", self.op_code(), arg0.to_u32()),
            Self::SetLast(arg0) => write!(f, "{:>2}:SetLast({})", self.op_code(), arg0.to_u32()),
            Self::Call(arg0, arg1) => {
                write!(
                    f,
                    "{:>2}:Call(nargs:{},f:{})",
                    self.op_code(),
                    arg0,
                    arg1.to_u32()
                )
            }
            Self::AllocObj(arg0, arg1) => {
                let (tag, objbit, entrynum) = arg1.decode();
                let ObjHeader(i) = arg1;
                write!(
                    f,
                    "{:>2}:AllocObj(max:{},header:{}[tag:{},objbit:{},entry:{}])",
                    self.op_code(),
                    arg0.to_u32(),
                    i,
                    tag,
                    objbit,
                    entrynum
                )
            }
            Self::Peek => write!(f, "{:>2}:Peek", self.op_code()),
            Self::Pop(u) => write!(f, "{:>2}:Pop({})", self.op_code(), u.to_u32()),
            Self::ObjTag => write!(f, "{:>2}:ObjTag", self.op_code()),
            Self::ObjField(u) => write!(f, "{:>2}:ObjField({})", self.op_code(), u.to_u32()),
            Self::AllocLocal(u) => write!(f, "{:>2}:AllocLocal({})", self.op_code(), u.to_u32()),
            Self::OutputAction(u) => {
                write!(f, "{:>2}:OutputAction({})", self.op_code(), u.to_u32())
            }
            Self::UpdateDev(u) => write!(f, "{:>2}:UpdateDev({})", self.op_code(), u.to_u32()),
            Self::EndUpdateNodeObj(u) => {
                write!(f, "{:>2}:EndUpdNodeObj({})", self.op_code(), u.to_u32())
            }
            Self::DropLocalObj(i) => {
                write!(f, "{:>2}:DropLocalObj({})", self.op_code(), i.to_i32())
            }
            Self::GetNodeRef(u) => write!(f, "{:>2}:GetNodeRef({})", self.op_code(), u.to_u32()),
            Self::GetDataRef(u) => write!(f, "{:>2}:GetDataRef({})", self.op_code(), u.to_u32()),
            Self::GetLocalRef(i) => write!(f, "{:>2}:GetLocalRef({})", self.op_code(), i.to_i32()),
            Self::SetLocalRef(i) => write!(f, "{:>2}:SetLocalRef({})", self.op_code(), i.to_i32()),
            Self::ObjFieldRef(u) => write!(f, "{:>2}:Pop({})", self.op_code(), u.to_u32()),
            Self::GetLastRef(u) => write!(f, "{:>2}:GetLastRef({})", self.op_code(), u.to_u32()),
            Self::SetLastRef(u) => write!(f, "{:>2}:SetLastRef({})", self.op_code(), u.to_u32()),
            Self::SetNodeRef(u) => write!(f, "{:>2}:SetNodeRef({})", self.op_code(), u.to_u32()),
            Self::SetDataRef(u) => write!(f, "{:>2}:SetDataRef({})", self.op_code(), u.to_u32()),
            Self::DropLast(u) => write!(f, "{:>2}:DropLast({})", self.op_code(), u.to_u32()),
            Self::Abort => write!(f, "{:>2}:Abort", self.op_code()),
        }
    }
}
#[derive(Clone, Eq, PartialEq)]
pub enum UnsignedNum {
    U8(u8),
    U16(u16),
    U32(u32),
}
#[derive(Clone, Eq, PartialEq)]
pub enum SignedNum {
    I8(i8),
    I16(i16),
    I32(i32),
}
impl UnsignedNum {
    pub fn to_u32(&self) -> u32 {
        match self {
            UnsignedNum::U8(u) => *u as u32,
            UnsignedNum::U32(u) => *u,
            UnsignedNum::U16(u) => *u as u32,
        }
    }
    pub fn from_u32(i: u32) -> Self {
        if u8::MIN as u32 <= i && i <= u8::MAX as u32 {
            UnsignedNum::U8(i.try_into().unwrap())
        } else if u16::MIN as u32 <= i && i <= u16::MAX as u32 {
            UnsignedNum::U16(i.try_into().unwrap())
        } else {
            UnsignedNum::U32(i)
        }
    }
    pub fn from_usize(i: usize) -> Option<Self> {
        if i <= u32::MAX as usize {
            Some(Self::from_u32(i.try_into().unwrap()))
        } else {
            None
        }
    }
}
impl SignedNum {
    pub fn to_i32(&self) -> i32 {
        match self {
            SignedNum::I8(i) => *i as i32,
            SignedNum::I16(i) => *i as i32,
            SignedNum::I32(i) => *i,
        }
    }
    pub fn from_i32(i: i32) -> Self {
        if i8::MIN as i32 <= i && i <= i8::MAX as i32 {
            SignedNum::I8(i.try_into().unwrap())
        } else if i16::MIN as i32 <= i && i <= i16::MAX as i32 {
            SignedNum::I16(i.try_into().unwrap())
        } else {
            SignedNum::I32(i)
        }
    }
    pub fn from_usize(i: usize) -> Option<Self> {
        if i <= i32::MAX as usize {
            Some(Self::from_i32(i.try_into().unwrap()))
        } else {
            None
        }
    }
}

impl Insn {
    pub fn op_code(&self) -> u8 {
        //opcode, len
        match self {
            Insn::None => 1,
            Insn::Nil => 2,
            Insn::Not => 3,
            Insn::Minus => 4,
            Insn::Add => 5,
            Insn::Sub => 6,
            Insn::Mul => 7,
            Insn::Div => 8,
            Insn::Mod => 9,
            Insn::ShiftL => 10,
            Insn::ShiftR => 11,
            Insn::Ls => 12,
            Insn::Leq => 13,
            Insn::Gt => 14,
            Insn::Geq => 15,
            Insn::Eq => 16,
            Insn::Neq => 17,

            Insn::BitAnd => 18,
            Insn::BitOr => 19,
            Insn::BitXor => 20,
            Insn::Return => 21,
            Insn::Print => 22,
            Insn::PrintObj => 23,
            Insn::Halt => 24,
            Insn::Peek => 25,
            Insn::Placeholder => panic!("placeholder must not appear"),
            Insn::PushTrue => 26,
            Insn::PushFalse => 27,
            Insn::Abort => 28,
            Insn::Int(n) => match n {
                SignedNum::I8(0) => 30,
                SignedNum::I8(1) => 31,
                SignedNum::I8(2) => 32,
                SignedNum::I8(3) => 33,
                SignedNum::I8(4) => 34,
                SignedNum::I8(5) => 35,
                SignedNum::I8(6) => 36,
                SignedNum::I8(_) => 37,
                SignedNum::I16(_) => 38,
                SignedNum::I32(_) => 39,
            },
            Insn::GetLocal(n) => match n {
                SignedNum::I8(0) => 40,
                SignedNum::I8(1) => 41,
                SignedNum::I8(2) => 42,
                SignedNum::I8(3) => 43,
                SignedNum::I8(4) => 44,
                SignedNum::I8(5) => 45,
                SignedNum::I8(6) => 46,
                SignedNum::I8(_) => 47,
                SignedNum::I16(_) => 48,
                SignedNum::I32(_) => 49,
            },
            Insn::SetLocal(n) => match n {
                SignedNum::I8(0) => 50,
                SignedNum::I8(1) => 51,
                SignedNum::I8(2) => 52,
                SignedNum::I8(3) => 53,
                SignedNum::I8(4) => 54,
                SignedNum::I8(5) => 55,
                SignedNum::I8(6) => 56,
                SignedNum::I8(_) => 57,
                SignedNum::I16(_) => 58,
                SignedNum::I32(_) => 59,
            },
            Insn::AllocLocal(n) => match n {
                UnsignedNum::U8(0) => panic!("emit_local"),
                UnsignedNum::U8(1) => 61,
                UnsignedNum::U8(2) => 62,
                UnsignedNum::U8(3) => 63,
                UnsignedNum::U8(4) => 64,
                UnsignedNum::U8(5) => 65,
                UnsignedNum::U8(6) => 66,
                UnsignedNum::U8(_) => 67,
                UnsignedNum::U16(_) => 68,
                UnsignedNum::U32(_) => 69,
            },
            Insn::Pop(n) => match n {
                UnsignedNum::U8(0) => 70,
                UnsignedNum::U8(1) => 71,
                UnsignedNum::U8(2) => 72,
                UnsignedNum::U8(3) => 73,
                UnsignedNum::U8(4) => 74,
                UnsignedNum::U8(5) => 75,
                UnsignedNum::U8(6) => 76,
                UnsignedNum::U8(_) => 77,
                UnsignedNum::U16(_) => 78,
                UnsignedNum::U32(_) => 79,
            },
            Insn::Jne8(_) => 80,
            Insn::Jne16(_) => 81,
            Insn::Jne32(_) => 82,
            Insn::Je8(_) => 83,
            Insn::Je16(_) => 84,
            Insn::Je32(_) => 85,
            Insn::J8(_) => 86,
            Insn::J16(_) => 87,
            Insn::J32(_) => 88,

            Insn::GetLast(n) => match n {
                UnsignedNum::U8(0) => 90,
                UnsignedNum::U8(1) => 91,
                UnsignedNum::U8(2) => 92,
                UnsignedNum::U8(3) => 93,
                UnsignedNum::U8(_) => 94,
                UnsignedNum::U16(_) => 95,
                UnsignedNum::U32(_) => 96,
            },
            Insn::SetNode(n) => match n {
                UnsignedNum::U8(_) => 97,
                UnsignedNum::U16(_) => 98,
                UnsignedNum::U32(_) => 99,
            },
            Insn::ObjField(n) => match n {
                UnsignedNum::U8(0) => 100,
                UnsignedNum::U8(1) => 101,
                UnsignedNum::U8(2) => 102,
                UnsignedNum::U8(3) => 103,
                UnsignedNum::U8(4) => 104,
                UnsignedNum::U8(5) => 105,
                UnsignedNum::U8(6) => 106,
                _ => panic!(),
            },

            Insn::UpdateDev(n) => match n {
                UnsignedNum::U8(0) => 110,
                UnsignedNum::U8(1) => 111,
                UnsignedNum::U8(2) => 112,
                UnsignedNum::U8(3) => 113,
                UnsignedNum::U8(_) => 114,
                UnsignedNum::U16(_) => panic!(),
                UnsignedNum::U32(_) => panic!(),
            },
            Insn::UpdateNode(n) => match n {
                UnsignedNum::U8(_) => 117,
                UnsignedNum::U16(_) => 118,
                UnsignedNum::U32(_) => 119,
            },
            Insn::OutputAction(n) => match n {
                UnsignedNum::U8(0) => 120,
                UnsignedNum::U8(1) => 121,
                UnsignedNum::U8(2) => 122,
                UnsignedNum::U8(3) => 123,
                UnsignedNum::U8(_) => 124,
                UnsignedNum::U16(_) => panic!(),
                UnsignedNum::U32(_) => panic!(),
            },

            Insn::Call(_, n) => match n {
                UnsignedNum::U8(_) => 127,
                UnsignedNum::U16(_) => 128,
                UnsignedNum::U32(_) => 129,
            },
            Insn::GetData(n) => match n {
                UnsignedNum::U8(_) => 130,
                UnsignedNum::U16(_) => 131,
                UnsignedNum::U32(_) => 132,
            },

            Insn::GetNode(n) => match n {
                UnsignedNum::U8(_) => 133,
                UnsignedNum::U16(_) => 134,
                UnsignedNum::U32(_) => 135,
            },

            Insn::SetData(n) => match n {
                UnsignedNum::U8(_) => 141,
                UnsignedNum::U16(_) => 142,
                UnsignedNum::U32(_) => 143,
            },
            Insn::ObjTag => 144,
            Insn::SetLast(n) => match n {
                UnsignedNum::U8(0) => 150,
                UnsignedNum::U8(1) => 151,
                UnsignedNum::U8(2) => 152,
                UnsignedNum::U8(3) => 153,
                UnsignedNum::U8(_) => 154,
                UnsignedNum::U16(_) => 155,
                UnsignedNum::U32(_) => 156,
            },
            Insn::EndUpdateNode(n) => match n {
                UnsignedNum::U8(_) => 157,
                UnsignedNum::U16(_) => 158,
                UnsignedNum::U32(_) => 159,
            },
            Insn::AllocObj(n, _) => match n {
                UnsignedNum::U8(0) => 160,
                UnsignedNum::U8(1) => 161,
                UnsignedNum::U8(2) => 162,
                UnsignedNum::U8(3) => 163,
                UnsignedNum::U8(4) => 164,
                UnsignedNum::U8(5) => 165,
                UnsignedNum::U8(6) => 166,
                UnsignedNum::U8(_) => 167,
                UnsignedNum::U16(_) => panic!("typecheck"),
                UnsignedNum::U32(_) => panic!("typecheck"),
            },
            Insn::DropLocalObj(i) => match i {
                SignedNum::I8(n @ 0..=6) => *n as u8 + 170,
                SignedNum::I8(_) => 177,
                SignedNum::I16(_) => 178,
                SignedNum::I32(_) => 179,
            },
            Insn::GetLocalRef(i) => match i {
                SignedNum::I8(n @ 0..=6) => *n as u8 + 180,
                SignedNum::I8(_) => 187,
                SignedNum::I16(_) => 188,
                SignedNum::I32(_) => 189,
            },
            Insn::SetLocalRef(i) => match i {
                SignedNum::I8(a @ 0..=6) => *a as u8 + 190,
                SignedNum::I8(_) => 197,
                SignedNum::I16(_) => 198,
                SignedNum::I32(_) => 199,
            },
            Insn::ObjFieldRef(u) => match u {
                UnsignedNum::U8(a @ 0..=6) => *a + 200,
                _ => panic!(),
            },
            Insn::EndUpdateNodeObj(u) => match u {
                UnsignedNum::U8(_) => 210,
                UnsignedNum::U16(_) => 211,
                UnsignedNum::U32(_) => 212,
            },

            Insn::GetNodeRef(u) => match u {
                UnsignedNum::U8(_) => 213,
                UnsignedNum::U16(_) => 214,
                UnsignedNum::U32(_) => 215,
            },
            Insn::GetDataRef(u) => match u {
                UnsignedNum::U8(_) => 216,
                UnsignedNum::U16(_) => 217,
                UnsignedNum::U32(_) => 218,
            },
            Insn::GetLastRef(u) => match u {
                UnsignedNum::U8(a @ 0..=3) => *a + 220,
                UnsignedNum::U8(_) => 224,
                UnsignedNum::U16(_) => 225,
                UnsignedNum::U32(_) => 226,
            },
            Insn::SetDataRef(u) => match u {
                UnsignedNum::U8(_) => 227,
                UnsignedNum::U16(_) => 228,
                UnsignedNum::U32(_) => 229,
            },
            Insn::SetLastRef(u) => match u {
                UnsignedNum::U8(a @ 0..=3) => *a + 230,
                UnsignedNum::U8(_) => 234,
                UnsignedNum::U16(_) => 235,
                UnsignedNum::U32(_) => 236,
            },
            Insn::SetNodeRef(u) => match u {
                UnsignedNum::U8(_) => 237,
                UnsignedNum::U16(_) => 238,
                UnsignedNum::U32(_) => 239,
            },
            Insn::DropLast(u) => match u {
                UnsignedNum::U8(_) => 240,
                UnsignedNum::U16(_) => 241,
                UnsignedNum::U32(_) => 242,
            },
            Insn::J0 => 243,
            Insn::J1 => 244,
            Insn::Je0 => 245,
            Insn::Je1 => 246,
            Insn::Jne0 => 247,
            Insn::Jne1 => 248,
        }
    }
    fn push_byte_code(&self, ret: &mut Vec<u8>) {
        ret.push(self.op_code());
        match self {
            // no immediate value
            Insn::None
            | Insn::Nil
            | Insn::Not
            | Insn::Minus
            | Insn::Add
            | Insn::Sub
            | Insn::Mul
            | Insn::Div
            | Insn::Mod
            | Insn::ShiftL
            | Insn::ShiftR
            | Insn::Ls
            | Insn::Leq
            | Insn::Gt
            | Insn::Geq
            | Insn::Eq
            | Insn::Neq
            | Insn::BitAnd
            | Insn::BitOr
            | Insn::BitXor
            | Insn::Return
            | Insn::Print
            | Insn::PrintObj
            | Insn::Halt
            | Insn::Peek
            | Insn::Placeholder
            | Insn::Abort
            | Insn::PushTrue
            | Insn::PushFalse => return,

            Insn::Int(n) => match n {
                SignedNum::I8(0..=6) => return,
                SignedNum::I8(i) => ret.push(i.to_le_bytes()[0]),
                SignedNum::I16(i) => push_i16_le(*i, ret),
                SignedNum::I32(i) => push_i32_le(*i, ret),
            },
            Insn::GetLocal(n) => match n {
                SignedNum::I8(0..=6) => return,
                SignedNum::I8(i) => ret.push(i.to_le_bytes()[0]),
                SignedNum::I16(i) => push_i16_le(*i, ret),
                SignedNum::I32(i) => push_i32_le(*i, ret),
            },
            Insn::SetLocal(n) => match n {
                SignedNum::I8(0..=6) => return,
                SignedNum::I8(i) => ret.push(i.to_le_bytes()[0]),
                SignedNum::I16(i) => push_i16_le(*i, ret),
                SignedNum::I32(i) => push_i32_le(*i, ret),
            },
            Insn::AllocLocal(n) => match n {
                UnsignedNum::U8(0..=6) => return,
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::Pop(n) => match n {
                UnsignedNum::U8(0..=6) => return,
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },

            Insn::GetLast(n) => match n {
                UnsignedNum::U8(0..=3) => return,
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::SetLast(n) => match n {
                UnsignedNum::U8(0..=3) => return,
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::SetNode(n) => match n {
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::ObjField(n) => match n {
                UnsignedNum::U8(0..=6) => return,
                _ => panic!("too many obj field"),
            },
            Insn::EndUpdateNode(n) => match n {
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::UpdateDev(n) => match n {
                UnsignedNum::U8(0..=3) => return,
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::UpdateNode(n) => match n {
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::OutputAction(n) => match n {
                UnsignedNum::U8(0..=3) => return,
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },

            Insn::Call(u, n) => {
                ret.push(*u);
                match n {
                    UnsignedNum::U8(u) => ret.push(*u),
                    UnsignedNum::U16(u) => push_u16_le(*u, ret),
                    UnsignedNum::U32(u) => push_u32_le(*u, ret),
                }
            }
            Insn::GetData(n) => match n {
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },

            Insn::GetNode(n) => match n {
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },

            Insn::SetData(n) => match n {
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::AllocObj(u, u2) => {
                match u {
                    UnsignedNum::U8(0..=6) => (),
                    UnsignedNum::U8(u) => ret.push(*u),
                    UnsignedNum::U16(_) => panic!("typecheck"),
                    UnsignedNum::U32(_) => panic!("typecheck"),
                }
                push_u32_le(u2.0, ret);
            }
            Insn::ObjTag => return,
            Insn::Jne8(i) | Insn::Je8(i) | Insn::J8(i) => ret.push(i.to_le_bytes()[0]),
            Insn::Jne16(i) | Insn::Je16(i) | Insn::J16(i) => push_i16_le(*i, ret),
            Insn::Jne32(i) | Insn::Je32(i) | Insn::J32(i) => push_i32_le(*i, ret),
            Insn::EndUpdateNodeObj(u) => match u {
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::DropLocalObj(i) => match i {
                SignedNum::I8(0..=6) => return,
                SignedNum::I8(i) => ret.push(i.to_le_bytes()[0]),
                SignedNum::I16(i) => push_i16_le(*i, ret),
                SignedNum::I32(i) => push_i32_le(*i, ret),
            },
            Insn::GetNodeRef(u) => match u {
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::GetDataRef(u) => match u {
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::SetDataRef(u) => match u {
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::SetNodeRef(u) => match u {
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::GetLocalRef(i) => match i {
                SignedNum::I8(0..=6) => return,
                SignedNum::I8(i) => ret.push(i.to_le_bytes()[0]),
                SignedNum::I16(i) => push_i16_le(*i, ret),
                SignedNum::I32(i) => push_i32_le(*i, ret),
            },
            Insn::SetLocalRef(i) => match i {
                SignedNum::I8(0..=6) => return,
                SignedNum::I8(i) => ret.push(i.to_le_bytes()[0]),
                SignedNum::I16(i) => push_i16_le(*i, ret),
                SignedNum::I32(i) => push_i32_le(*i, ret),
            },
            Insn::ObjFieldRef(u) => match u {
                UnsignedNum::U8(0..=6) => return,
                _ => panic!("too many obj field"),
            },
            Insn::GetLastRef(u) => match u {
                UnsignedNum::U8(0..=3) => return,
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::SetLastRef(u) => match u {
                UnsignedNum::U8(0..=3) => return,
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::DropLast(n) => match n {
                UnsignedNum::U8(u) => ret.push(*u),
                UnsignedNum::U16(u) => push_u16_le(*u, ret),
                UnsignedNum::U32(u) => push_u32_le(*u, ret),
            },
            Insn::J0 | Insn::J1 | Insn::Je0 | Insn::Je1 | Insn::Jne0 | Insn::Jne1 => return,
        };
    }
}

fn push_i32_le(i: i32, ret: &mut Vec<u8>) {
    for b in i.to_le_bytes() {
        ret.push(b)
    }
}
fn push_i16_le(i: i16, ret: &mut Vec<u8>) {
    for b in i.to_le_bytes() {
        ret.push(b)
    }
}
fn push_u32_le(i: u32, ret: &mut Vec<u8>) {
    for b in i.to_le_bytes() {
        ret.push(b)
    }
}
fn push_u16_le(i: u16, ret: &mut Vec<u8>) {
    for b in i.to_le_bytes() {
        ret.push(b)
    }
}

impl Insn {
    pub fn j(i: i32) -> Self {
        if i == 0 {
            Insn::J0
        } else if i == 1 {
            Insn::J1
        } else if i8::MIN as i32 <= i && i <= i8::MAX as i32 {
            Insn::J8(i as i8)
        } else {
            Insn::J32(i as i32)
        }
    }
    pub fn je(i: i32) -> Self {
        if i == 0 {
            Insn::Je0
        } else if i == 1 {
            Insn::Je1
        } else if i8::MIN as i32 <= i && i <= i8::MAX as i32 {
            Insn::Je8(i as i8)
        } else {
            Insn::Je32(i as i32)
        }
    }
    pub fn jne(i: i32) -> Self {
        if i == 0 {
            Insn::Jne0
        } else if i == 1 {
            Insn::Jne1
        } else if i8::MIN as i32 <= i && i <= i8::MAX as i32 {
            Insn::Jne8(i as i8)
        } else {
            Insn::Jne32(i as i32)
        }
    }
}
pub fn bytecode_len(v: &[Insn]) -> usize {
    let mut ret = 0;
    for insn in v {
        ret += match &insn {
            // no immediate value
            Insn::None
            | Insn::Nil
            | Insn::Not
            | Insn::Minus
            | Insn::Add
            | Insn::Sub
            | Insn::Mul
            | Insn::Div
            | Insn::Mod
            | Insn::ShiftL
            | Insn::ShiftR
            | Insn::Ls
            | Insn::Leq
            | Insn::Gt
            | Insn::Geq
            | Insn::Eq
            | Insn::Neq
            | Insn::BitAnd
            | Insn::BitOr
            | Insn::BitXor
            | Insn::Return
            | Insn::Print
            | Insn::PrintObj
            | Insn::Halt
            | Insn::Peek
            | Insn::Placeholder
            | Insn::Abort
            | Insn::PushTrue
            | Insn::PushFalse => 1,

            Insn::Int(n) => match n {
                SignedNum::I8(0..=6) => 1,
                SignedNum::I8(_) => 2,
                SignedNum::I16(_) => 3,
                SignedNum::I32(_) => 5,
            },
            Insn::GetLocal(n) => match n {
                SignedNum::I8(0..=6) => 1,
                SignedNum::I8(_) => 2,
                SignedNum::I16(_) => 3,
                SignedNum::I32(_) => 5,
            },
            Insn::SetLocal(n) => match n {
                SignedNum::I8(0..=6) => 1,
                SignedNum::I8(_) => 2,
                SignedNum::I16(_) => 3,
                SignedNum::I32(_) => 5,
            },
            Insn::AllocLocal(n) => match n {
                UnsignedNum::U8(0..=6) => 1,
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::Pop(n) => match n {
                UnsignedNum::U8(0..=6) => 1,
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },

            Insn::GetLast(n) => match n {
                UnsignedNum::U8(0..=3) => 1,
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::SetLast(n) => match n {
                UnsignedNum::U8(0..=3) => 1,
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::SetNode(n) => match n {
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::ObjField(n) => match n {
                UnsignedNum::U8(0..=6) => 1,
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::EndUpdateNode(n) => match n {
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::UpdateDev(n) => match n {
                UnsignedNum::U8(0..=3) => 1,
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::UpdateNode(n) => match n {
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::OutputAction(n) => match n {
                UnsignedNum::U8(0..=3) => 1,
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },

            Insn::Call(_, n) => match n {
                UnsignedNum::U8(_) => 3,
                UnsignedNum::U16(_) => 4,
                UnsignedNum::U32(_) => 6,
            },
            Insn::GetData(n) => match n {
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },

            Insn::GetNode(n) => match n {
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },

            Insn::SetData(n) => match n {
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::AllocObj(u, _) => match u {
                UnsignedNum::U8(0..=6) => 5,
                UnsignedNum::U8(_) => 6,
                UnsignedNum::U16(_) => panic!("typecheck"),
                UnsignedNum::U32(_) => panic!("typecheck"),
            },
            Insn::ObjTag => 1,
            Insn::Jne8(_) | Insn::Je8(_) | Insn::J8(_) => 2,
            Insn::Jne16(_) | Insn::Je16(_) | Insn::J16(_) => 3,
            Insn::Jne32(_) | Insn::Je32(_) | Insn::J32(_) => 5,
            Insn::EndUpdateNodeObj(u) => match u {
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::DropLocalObj(i) => match i {
                SignedNum::I8(0..=6) => 1,
                SignedNum::I8(_) => 2,
                SignedNum::I16(_) => 3,
                SignedNum::I32(_) => 5,
            },
            Insn::GetNodeRef(u) => match u {
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::GetDataRef(u) => match u {
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::GetLocalRef(i) => match i {
                SignedNum::I8(0..=6) => 1,
                SignedNum::I8(_) => 2,
                SignedNum::I16(_) => 3,
                SignedNum::I32(_) => 5,
            },
            Insn::SetLocalRef(i) => match i {
                SignedNum::I8(0..=6) => 1,
                SignedNum::I8(_) => 2,
                SignedNum::I16(_) => 3,
                SignedNum::I32(_) => 5,
            },
            Insn::ObjFieldRef(u) => match u {
                UnsignedNum::U8(0..=6) => 1,
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::GetLastRef(u) => match u {
                UnsignedNum::U8(0..=6) => 1,
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::SetLastRef(u) => match u {
                UnsignedNum::U8(0..=6) => 1,
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::SetNodeRef(u) => match u {
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::SetDataRef(u) => match u {
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::DropLast(u) => match u {
                UnsignedNum::U8(_) => 2,
                UnsignedNum::U16(_) => 3,
                UnsignedNum::U32(_) => 5,
            },
            Insn::J0 | Insn::J1 | Insn::Je0 | Insn::Je1 | Insn::Jne0 | Insn::Jne1 => 1,
        };
    }
    ret
}
#[derive(Clone, Eq, PartialEq)]
pub struct ObjHeader(pub u32);
impl ObjHeader {
    // header tag:7bit/ reserved:1bit/ numentry:3bit/ objbit:7bit/ refcnt:14bit
    pub fn new(tag: u32, objbit: &Vec<bool>, n_entry: u32) -> Self {
        let mut header = 0u32;
        header |= tag << 25;
        for (i, b) in objbit.iter().enumerate() {
            if *b {
                header |= 1 << (14 + i)
            }
        }
        header |= n_entry << 21;
        Self(header + 1)
    }
    pub fn decode(&self) -> (u32, String, u32) {
        let ObjHeader(i) = self;
        (
            i >> 25,                          //tag
            format!("{:b}", (i << 11) >> 25), //objbit
            (i << 8) >> 29,                   //numentry
        )
    }
}
