use crate::ast::{Expr, Stmt};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct StructDefInfo {
    pub name: String,
    pub fields: Vec<String>,
    pub field_types: Vec<Option<String>>,
    pub defaults: Vec<Option<Expr>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternFnInfo {
    pub abi: String,
    pub lib: Option<String>,
    pub name: String,
    pub return_type: Option<String>,
    pub params_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarType {
    Number(i32),         // Stack offset for numeric (integer) variables
    Float(i32),          // Stack offset for floating-point (f64) variables
    StringLabel(String), // Rodata label for string literals
    StringOffset(i32),   // Stack offset for string pointers
    Array(i32),          // Stack offset for array pointers
    Map(i32),            // Stack offset for map pointers
    Null(i32),           // Stack offset for null variables
    Struct { struct_name: String, offset: i32 },
    Interface { interface_name: String, offset: i32 },
}

#[derive(Debug, Clone)]
pub struct InterfaceDefInfo {
    pub name: String,
    pub methods: Vec<crate::ast::InterfaceMethod>,
    pub embedded: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ScopeState {
    pub variables: HashMap<String, VarType>,
    pub stack_offset: i32,
    pub loop_stack: Vec<(String, String, i32)>,
    pub active_defers: Vec<(usize, Stmt, i32)>,
    pub next_defer_idx: usize,
    pub current_fn_name: String,
}

#[derive(Debug, Default)]
pub struct CodeGenContext {
    pub label_counter: usize,
    pub string_counter: usize,
    pub variables: HashMap<String, VarType>,
    pub structs: HashMap<String, StructDefInfo>,
    pub interfaces: HashMap<String, InterfaceDefInfo>,
    pub vtables: HashMap<(String, String), String>,
    pub stack_offset: i32,
    pub loop_stack: Vec<(String, String, i32)>,
    pub functions: HashSet<String>,
    pub extern_functions: HashMap<String, ExternFnInfo>,
    pub extern_libs: HashSet<String>,
    pub active_defers: Vec<(usize, Stmt, i32)>,
    pub next_defer_idx: usize,
    pub current_fn_name: String,
    pub globals: HashMap<String, (String, Option<String>)>,
}

impl CodeGenContext {
    pub fn new() -> Self {
        Self {
            label_counter: 0,
            string_counter: 0,
            variables: HashMap::new(),
            structs: HashMap::new(),
            interfaces: HashMap::new(),
            vtables: HashMap::new(),
            stack_offset: 0,
            loop_stack: Vec::new(),
            functions: HashSet::new(),
            extern_functions: HashMap::new(),
            extern_libs: HashSet::new(),
            active_defers: Vec::new(),
            next_defer_idx: 0,
            current_fn_name: String::new(),
            globals: HashMap::new(),
        }
    }

    pub fn next_label(&mut self) -> String {
        let label = format!(".L{}", self.label_counter);
        self.label_counter += 1;
        label
    }

    pub fn next_string_label(&mut self) -> String {
        let label = format!("str_{}", self.string_counter);
        self.string_counter += 1;
        label
    }

    pub fn push_loop(&mut self, continue_lbl: String, break_lbl: String, base_stack_offset: i32) {
        self.loop_stack
            .push((continue_lbl, break_lbl, base_stack_offset));
    }

    pub fn pop_loop(&mut self) -> Option<(String, String, i32)> {
        self.loop_stack.pop()
    }

    pub fn current_loop(&self) -> Option<&(String, String, i32)> {
        self.loop_stack.last()
    }

    pub fn enter_function(&mut self) -> ScopeState {
        let mut fn_vars = HashMap::new();
        for (k, v) in &self.variables {
            if self.globals.contains_key(k)
                || k.starts_with("map_field_str:")
                || k.starts_with("map_str:")
                || k.starts_with("fn_ret_str:")
                || k.starts_with("fn_ret_str_arr:")
                || k.starts_with("fn_ret_flt:")
                || k.starts_with("fn_ret_struct:")
                || k.starts_with("fn_ret_tuple_str:")
                || k.starts_with("tuple_elem_str:")
                || k.starts_with("tuple_elem_flt:")
                || k.starts_with("struct_field_str:")
                || k.starts_with("struct_field_flt:")
                || k.starts_with("struct_field_arr:")
                || k.starts_with("struct_field_map:")
                || k.starts_with("struct_field_struct:")
                || k.starts_with("arr_is_str:")
                || k.starts_with("arr_is_flt:")
                || k.starts_with("fn_param_str:")
                || k.starts_with("fn_param_str_arr:")
                || k.starts_with("channel_elem_str:")
                || k.starts_with("fn_param_interface:")
            {
                fn_vars.insert(k.clone(), v.clone());
            }
        }
        let saved = ScopeState {
            variables: std::mem::replace(&mut self.variables, fn_vars),
            stack_offset: self.stack_offset,
            loop_stack: std::mem::take(&mut self.loop_stack),
            active_defers: std::mem::take(&mut self.active_defers),
            next_defer_idx: self.next_defer_idx,
            current_fn_name: std::mem::take(&mut self.current_fn_name),
        };
        self.stack_offset = 0;
        self.next_defer_idx = 0;
        saved
    }

    pub fn exit_function(&mut self, saved: ScopeState) {
        self.variables = saved.variables;
        self.stack_offset = saved.stack_offset;
        self.loop_stack = saved.loop_stack;
        self.active_defers = saved.active_defers;
        self.next_defer_idx = saved.next_defer_idx;
        self.current_fn_name = saved.current_fn_name;
    }
}
