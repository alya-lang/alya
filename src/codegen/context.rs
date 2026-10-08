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
    pub params: Vec<Option<String>>,
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
    pub nullable_heap_vars: HashSet<String>,
    pub nonescaping_params: HashSet<String>,
}

/// Counters for the return-tag protocol measurement
/// (alya-lang/alya#55-C): how many functions qualify (supply) vs how
/// many user-function call sites trust the tag (hits) or fall back to
/// legacy classification (misses, recorded by callee name so the report
/// can split forward-reference misses from structural ones). Inert
/// unless `ALYA_TAG_STATS` is set; never affects emission.
#[derive(Debug, Default)]
pub struct TagStats {
    pub markers: u64,
    pub call_hits: u64,
    pub miss_names: Vec<String>,
    /// Misses emitted outside any function body (top-level flow always
    /// precedes all functions, so these can never see markers).
    pub miss_top_level: u64,
    /// Resolved once per compilation from `ALYA_TAG_STATS`; all
    /// recording short-circuits on false, so disabled builds pay one
    /// branch per call site and no allocations.
    pub enabled: bool,
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
    /// Declared function arities: name -> (required_params, total_params).
    /// Used to keep builtin `assert`/`assert_eq` desugaring from shadowing
    /// (or being shadowed by) user functions with incompatible arity.
    pub fn_arities: HashMap<String, (usize, usize)>,
    pub extern_functions: HashMap<String, ExternFnInfo>,
    pub extern_libs: HashSet<String>,
    pub active_defers: Vec<(usize, Stmt, i32)>,
    pub next_defer_idx: usize,
    pub current_fn_name: String,
    pub globals: HashMap<String, (String, Option<String>)>,
    pub enums: HashSet<String>,
    pub tag_stats: TagStats,
    /// Locals proven to hold only `null` or heap values
    /// (see `nullable_heap_locals`): their reads and rebind-releases
    /// take the probe-free direct calls. Recomputed per function body.
    pub nullable_heap_vars: HashSet<String>,
    /// Heap params that never escape the frame (see
    /// `nonescaping_params`): no entry retain and no scope release,
    /// skipped as a balanced pair. Recomputed per function body.
    pub nonescaping_params: HashSet<String>,
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
            fn_arities: HashMap::new(),
            extern_functions: HashMap::new(),
            extern_libs: HashSet::new(),
            active_defers: Vec::new(),
            next_defer_idx: 0,
            current_fn_name: String::new(),
            globals: HashMap::new(),
            enums: HashSet::new(),
            tag_stats: TagStats::default(),
            nullable_heap_vars: HashSet::new(),
            nonescaping_params: HashSet::new(),
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
                || k.starts_with("map_nonstr:")
                || k.starts_with("map_nonstr_key:")
                || k.starts_with("fn_ret_str:")
                || k.starts_with("fn_ret_str_arr:")
                || k.starts_with("fn_ret_arr:")
                || k.starts_with("fn_ret_map:")
                || k.starts_with("fn_ret_flt:")
                || k.starts_with("fn_ret_flt_ann:")
                || k.starts_with("fn_ret_int:")
                || k.starts_with("fn_ret_tagged:")
                || k.starts_with("fn_ret_tuple_flt:")
                || k.starts_with("fn_ret_tuple_struct:")
                || k.starts_with("fn_ret_struct:")
                || k.starts_with("fn_ret_fresh:")
                || k.starts_with("fn_ret_tuple_str:")
                || k.starts_with("tuple_elem_str:")
                || k.starts_with("tuple_elem_flt:")
                || k.starts_with("tuple_elem_struct:")
                || k.starts_with("struct_field_str:")
                || k.starts_with("struct_field_flt:")
                || k.starts_with("struct_field_mixed:")
                || k.starts_with("map_field_flt:")
                || k.starts_with("map_flt:")
                || k.starts_with("struct_field_arr:")
                || k.starts_with("struct_field_arr_int:")
                || k.starts_with("struct_field_map:")
                || k.starts_with("struct_field_struct:")
                || k.starts_with("arr_is_str:")
                || k.starts_with("arr_nonstr:")
                || k.starts_with("var_is_uint:")
                || k.starts_with("arr_is_flt:")
                || k.starts_with("arr_flt_ann:")
                || k.starts_with("arr_nonflt:")
                || k.starts_with("arr_nonflt:")
                || k.starts_with("arr_is_int:")
                || k.starts_with("arr_struct_type:")
                || k.starts_with("fn_param_str:")
                || k.starts_with("fn_param_str_arr:")
                || k.starts_with("fn_param_flt:")
                || k.starts_with("fn_param_arr:")
                || k.starts_with("channel_elem_str:")
                || k.starts_with("fn_param_interface:")
                // Ambiguity sentinels (#101) must survive into function
                // bodies: in-body recording sites consult them.
                || k.starts_with("fn_ambiguous:")
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
            nullable_heap_vars: std::mem::take(&mut self.nullable_heap_vars),
            nonescaping_params: std::mem::take(&mut self.nonescaping_params),
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
        self.nullable_heap_vars = saved.nullable_heap_vars;
        self.nonescaping_params = saved.nonescaping_params;
    }
}
