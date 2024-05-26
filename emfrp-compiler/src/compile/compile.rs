use std::collections::HashSet;

use crate::ast::*;
use crate::compile::compile_common::*;
use crate::compile::typed_ast::*;
use crate::insn::*;

impl Compiler {
    //if input only contains type definition,
    //no information needs to be sent from host computer to machine
    //in that case, this function returns Ok(None)
    pub fn compile(&mut self, prog: Program) -> Result<Option<CompiledCode>, CompileErr> {
        match prog {
            Program::Def(defs) => {
                self.compile_type_def(&defs)?;
                let defs: Vec<VarDef> = defs
                    .into_iter()
                    .filter_map(|def| match def {
                        Def::Type(_) => None,
                        Def::Var(v) => Some(v),
                    })
                    .collect();

                if defs.len() == 0 {
                    return Ok(None);
                }
                let res = self.compile_var_def(defs)?;
                for nd in &mut self.node_info {
                    nd.is_new = false;
                    nd.has_value = true;
                }
                for f in &mut self.func_info {
                    f.is_new = false;
                }
                for data in &mut self.data_info {
                    data.is_new = false;
                }
                Ok(Some(res))
            }
            Program::Exp(e) => {
                let (t, init) = self.compile_exp(e)?;
                Ok(Some(CompiledCode::Eval(t, init)))
            }
        }
    }
    pub fn compile_exp(&mut self, e: Exp) -> Result<(Type, Vec<Insn>), CompileErr> {
        let typed_exp = self.exp_type_check(e)?;
        let mut e = self.emit_code_exp(&typed_exp)?;
        if typed_exp.get_type().is_obj_type() {
            e.push(Insn::PrintObj);
        } else {
            e.push(Insn::Print);
        }
        e.push(Insn::Halt);

        Ok((typed_exp.get_type().clone(), e))
    }
    pub fn compile_type_def(&mut self, defs: &Vec<Def>) -> Result<(), CompileErr> {
        for def in defs {
            if let Def::Type(TypeDef { name, variants }) = def {
                if self.types.contains_key(name) {
                    return Err(CompileErr::TypeAlreadyExists);
                } else {
                    let s = match name {
                        TypeName::User(s) => s,
                        _ => return Err(CompileErr::TypeAlreadyExists),
                    };

                    let mut vars = Vec::with_capacity(variants.len());
                    for Variant { constructor, elems } in variants {
                        if elems.len() > 7 {
                            return Err(CompileErr::TooManyFields);
                        }
                        let mut types = Vec::with_capacity(elems.len());
                        for tname in elems {
                            let typ = self.get_type_with_type_name(&tname)?;
                            types.push(typ);
                        }
                        vars.push((constructor.clone(), types));
                    }
                    self.types
                        .insert(name.clone(), Type::User(s.to_string(), vars));
                }
            }
        }
        Ok(())
    }

    pub fn compile_var_def(&mut self, defs: Vec<VarDef>) -> Result<CompiledCode, CompileErr> {
        let node_def_included = defs.iter().any(|def| matches!(def, VarDef::Node { .. }));
        self.register_nodes(&defs)?;
        self.register_vars(&defs)?;
        let n_new_nodes = self.node_info.iter().filter(|nd| nd.is_new).count();
        let n_new_func = self.func_info.iter().filter(|f| f.is_new).count();
        let n_new_data = self.data_info.iter().filter(|d| d.is_new).count();

        let tdefs = defs
            .into_iter()
            .map(|d| self.vardef_type_check(d))
            .collect::<CResult<Vec<TVarDef>>>()?;
        self.add_dependency(&tdefs)?;
        let node = self.emit_code_def_nodes(&tdefs)?;
        let func = self.emit_code_def_funcs(&tdefs)?;

        let save_last = self.atlast_manager.atlast_offset();
        let n_last = save_last.len();
        let upd_order = if node_def_included {
            self.topological_sort()?
        } else {
            vec![]
        };
        let mut update = vec![];
        for (i, &nd_i) in save_last.iter().enumerate() {
            match nd_i {
                None => (),
                Some(nd_i) => {
                    if self.node_info[nd_i].typ.is_obj_type() {
                        update.push(Insn::GetNodeRef(UnsignedNum::from_usize(nd_i).unwrap()));
                        update.push(Insn::SetLast(UnsignedNum::from_usize(i).unwrap()));
                    } else {
                        update.push(Insn::GetNode(UnsignedNum::from_usize(nd_i).unwrap()));
                        update.push(Insn::SetLast(UnsignedNum::from_usize(i).unwrap()));
                    }
                }
            }
        }
        for nd_i in upd_order {
            match self.node_info[nd_i].input_kind {
                NodeInputKind::None => continue,
                NodeInputKind::Dev => {
                    update.push(Insn::UpdateDev(UnsignedNum::from_usize(nd_i).unwrap()))
                }
                NodeInputKind::User => {
                    update.push(Insn::UpdateNode(UnsignedNum::from_usize(nd_i).unwrap()))
                }
            }
        }
        for (i, &nd_i) in save_last.iter().enumerate() {
            match nd_i {
                None => (),
                Some(nd_i) => {
                    if self.node_info[nd_i].typ.is_obj_type() {
                        update.push(Insn::DropLast(UnsignedNum::from_usize(i).unwrap()));
                    }
                }
            }
        }
        if !update.is_empty() {
            update.push(Insn::Halt);
        }

        let init = self.emit_code_init(&tdefs)?;

        Ok(CompiledCode::Def(BcDefVar {
            n_new_nodes,
            n_new_func,
            n_new_data,
            n_last,
            init,
            node,
            func,
            update,
        }))
    }

    fn register_nodes(&mut self, defs: &Vec<VarDef>) -> CResult<()> {
        for def in defs {
            match def {
                VarDef::Data { .. } | VarDef::Func { .. } => continue,
                VarDef::Node {
                    name,
                    typ,
                    init: _,
                    val: _,
                } => {
                    let t = self.get_type_with_type_name(typ)?;
                    if let Some(i) = self.node_offset(name) {
                        if &self.node_info[i].typ != &t {
                            let mut dep_list = vec![];
                            for nd in &self.node_info {
                                if nd.prev.contains(&i) {
                                    dep_list.push(nd.name.0.clone());
                                }
                            }
                            if dep_list.iter().any(|id| {
                                defs.iter()
                                    .find(|def| match def {
                                        VarDef::Node {
                                            name,
                                            typ: _,
                                            init: _,
                                            val: _,
                                        } => &name.0 == id,
                                        _ => false,
                                    })
                                    .is_none()
                            }) {
                                return Err(CompileErr::ConflictNodeType(name.0.clone(), dep_list));
                            }
                        }
                    }
                }
            }
        }
        for def in defs {
            match def {
                VarDef::Data { .. } | VarDef::Func { .. } => continue,
                VarDef::Node {
                    name,
                    typ,
                    init,
                    val: _,
                } => {
                    let t = self.get_type_with_type_name(typ)?;
                    let mut nd = NodeInfo {
                        name: name.clone(),
                        typ: t.clone(),
                        prev: HashSet::new(),
                        is_new: false,
                        has_value: init.is_some(),
                        atlast: HashSet::new(),
                        output_offset: None,
                        input_kind: NodeInputKind::User,
                    };

                    match self.node_offset(name) {
                        Some(i) => {
                            if matches!(self.node_info[i].input_kind, NodeInputKind::Dev) {
                                return Err(CompileErr::OverwriteDevInput);
                            }
                            nd.output_offset = self.node_info[i].output_offset;
                            std::mem::swap(&mut nd, &mut self.node_info[i]);
                            self.unregister_node(nd);
                        }
                        None => {
                            nd.is_new = true;
                            self.node_info.push(nd)
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn unregister_node(
        &mut self,
        NodeInfo {
            name: _,
            typ: _,
            prev: _,
            atlast,
            is_new: _,
            has_value: _,
            output_offset: _,
            input_kind: _,
        }: NodeInfo,
    ) {
        for nd in atlast {
            self.atlast_manager.remove_atlast_refcnt(nd);
        }
    }
    fn register_vars(&mut self, defs: &Vec<VarDef>) -> CResult<()> {
        for def in defs {
            match def {
                VarDef::Node { .. } => continue, // node must be registerd already
                VarDef::Data { name, typ, val: _ } => {
                    let t = self.get_type_with_type_name(typ)?;
                    let mut data = DataInfo {
                        name: name.clone(),
                        typ: t.clone(),
                        is_new: false,
                    };
                    match self.data_offset(name) {
                        Some(i) => self.data_info[i] = data,
                        None => {
                            data.is_new = true;
                            self.data_info.push(data)
                        }
                    }
                }
                VarDef::Func {
                    name,
                    ret,
                    params,
                    body: _,
                } => {
                    let mut prms = Vec::with_capacity(params.len());
                    for (id, typ) in params {
                        let t = self.get_type_with_type_name(typ)?;
                        prms.push((id.clone(), t.clone()))
                    }
                    let t = self.get_type_with_type_name(ret)?;
                    let mut func = FuncInfo {
                        name: name.clone(),
                        prms,
                        ret: t.clone(),
                        is_new: false,
                    };
                    match self.func_offset(name) {
                        Some(i) => self.func_info[i] = func,
                        None => {
                            func.is_new = true;
                            self.func_info.push(func)
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
