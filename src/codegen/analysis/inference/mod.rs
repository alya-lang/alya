pub mod arrays;
pub mod common;
pub mod floats;
pub mod maps;
pub mod strings;
pub mod structs;

pub use arrays::*;
pub use common::*;
pub use floats::*;
pub use maps::*;
pub use strings::*;
pub use structs::*;

use crate::ast::Program;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ProgramInference {
    pub known_strings: HashSet<String>,
    pub known_floats: HashSet<String>,
    pub known_arrays: HashSet<String>,
    pub known_maps: HashSet<String>,
    pub struct_inf: StructInference,
}

impl ProgramInference {
    pub fn analyze(program: &Program) -> Self {
        let call_index = crate::codegen::analysis::traversal::CallIndex::build(&program.statements);
        let known_strings = collect_known_string_vars_with_index(program, &call_index);
        let known_floats = collect_known_float_vars_with_index(program, &call_index);
        let known_arrays = collect_known_array_vars_with_index(program, &call_index);
        let known_maps = collect_known_map_vars_with_index(program, &call_index);
        let struct_inf = StructInference::analyze(program);
        Self {
            known_strings,
            known_floats,
            known_arrays,
            known_maps,
            struct_inf,
        }
    }

    #[inline]
    pub fn infer_param_is_string(
        &self,
        func_name: &str,
        param_idx: usize,
        program: &Program,
    ) -> bool {
        infer_param_is_string_with(func_name, param_idx, program, &self.known_strings)
    }

    #[inline]
    pub fn infer_param_is_string_array(
        &self,
        func_name: &str,
        param_idx: usize,
        program: &Program,
    ) -> bool {
        infer_param_is_string_array_with(func_name, param_idx, program, &self.known_strings)
    }

    #[inline]
    pub fn infer_param_is_float(
        &self,
        func_name: &str,
        param_idx: usize,
        program: &Program,
    ) -> bool {
        infer_param_is_float_with(func_name, param_idx, program, &self.known_floats)
    }

    #[inline]
    pub fn infer_param_is_float_array(
        &self,
        func_name: &str,
        param_idx: usize,
        program: &Program,
    ) -> bool {
        infer_param_is_float_array_with(func_name, param_idx, program, &self.known_floats)
    }

    #[inline]
    pub fn infer_param_is_array(
        &self,
        func_name: &str,
        param_idx: usize,
        program: &Program,
    ) -> bool {
        infer_param_is_array_with(func_name, param_idx, program, &self.known_arrays)
    }

    #[inline]
    pub fn infer_param_is_map(&self, func_name: &str, param_idx: usize, program: &Program) -> bool {
        infer_param_is_map_with(func_name, param_idx, program, &self.known_maps)
    }

    #[inline]
    pub fn infer_param_struct_type(&self, func_name: &str, param_idx: usize) -> Option<String> {
        let bare = func_name.rsplit("::").next().unwrap_or(func_name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        self.struct_inf
            .fn_params
            .get(&(func_name.to_string(), param_idx))
            .or_else(|| {
                self.struct_inf
                    .fn_params
                    .get(&(bare.to_string(), param_idx))
            })
            .cloned()
    }

    #[inline]
    pub fn infer_function_return_struct_type(&self, func_name: &str) -> Option<String> {
        let bare = func_name.rsplit("::").next().unwrap_or(func_name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        self.struct_inf
            .fn_returns
            .get(func_name)
            .or_else(|| self.struct_inf.fn_returns.get(bare))
            .cloned()
    }
}
