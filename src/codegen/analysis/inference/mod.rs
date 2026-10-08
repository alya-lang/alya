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
use std::collections::{HashMap, HashSet};

/// Struct-field kind families for contradiction analysis
/// (alya-lang/alya#132). A bare or qualified marker is sound only while
/// its field carries markers from a single family.
const FIELD_FAM_STR: u8 = 1;
const FIELD_FAM_ARR: u8 = 2;
const FIELD_FAM_MAP: u8 = 4;
const FIELD_FAM_FLT: u8 = 8;

fn field_marker_family(key: &str) -> Option<(u8, &str)> {
    // Longest (sub-kind) prefixes first: `struct_field_arr_str:` etc.
    // refine array elements but are still array-kind markers.
    for (prefix, fam) in [
        ("struct_field_arr_str:", FIELD_FAM_ARR),
        ("struct_field_arr_flt:", FIELD_FAM_ARR),
        ("struct_field_arr_int:", FIELD_FAM_ARR),
        ("struct_field_arr:", FIELD_FAM_ARR),
        ("struct_field_str:", FIELD_FAM_STR),
        ("struct_field_map:", FIELD_FAM_MAP),
        ("struct_field_flt:", FIELD_FAM_FLT),
    ] {
        if let Some(rest) = key.strip_prefix(prefix) {
            return Some((fam, rest));
        }
    }
    None
}

fn bare_struct_name(s: &str) -> &str {
    let b = s.rsplit("::").next().unwrap_or(s);
    b.rsplit("__").next().unwrap_or(b)
}

/// Removes every struct-field kind marker for fields whose markers span
/// two or more kind families, and returns the touched field names so the
/// string fixpoint can be re-run with their sentinels present.
///
/// Why this exists (alya-lang/alya#132): when every construction site
/// passes a *variable*, literal-kind observation sees DYN+DYN and emits
/// no sentinel, yet each family's dataflow still records its marker
/// (`struct_field_str:F.payload` on the text path,
/// `struct_field_arr:F.payload` on the byte path). Worse, markers
/// derived from those reads during the same fixpoint (`fn_param_str`,
/// `fn_ret_str`, per-variable field keys) go stale: deleting struct
/// markers after the fact cannot un-derive them (a stale `fn_ret_str`
/// once routed an array through `alya_str_store` and segfaulted on the
/// next read). Re-running the string pass with the sentinels present
/// suppresses both the markers and their derivations at the source.
///
/// Scope of removal mirrors the codegen-side cleanup: qualified keys go
/// only for groups where one struct's field is genuinely mixed (typed
/// reads of single-kind structs stay precise); bare-global keys go
/// whenever the bare name is contested (untyped reads demote to
/// dynamic). Callers re-run the string fixpoint with the returned
/// fields as sentinel seeds.
pub fn suppress_contradictory_struct_field_markers_inference(
    known_strings: &mut HashSet<String>,
    known_floats: &mut HashSet<String>,
    known_arrays: &mut HashSet<String>,
    known_maps: &mut HashSet<String>,
) -> HashSet<String> {
    // field -> kind mask (bare-global markers).
    let mut bare_fams: HashMap<String, u8> = HashMap::new();
    // (bare struct, field) -> kind mask (qualified markers).
    let mut qual_fams: HashMap<(String, String), u8> = HashMap::new();
    // (bare struct, field) -> (set index, key) for qualified removal.
    let mut qual_keys: HashMap<(String, String), Vec<(usize, String)>> = HashMap::new();
    // field -> (set index, key) for bare removal.
    let mut bare_keys: HashMap<String, Vec<(usize, String)>> = HashMap::new();

    for (idx, set) in [
        &*known_strings,
        &*known_floats,
        &*known_arrays,
        &*known_maps,
    ]
    .into_iter()
    .enumerate()
    {
        for key in set.iter() {
            let Some((fam, rest)) = field_marker_family(key) else {
                continue;
            };
            if let Some((s, f)) = rest.rsplit_once('.') {
                let group = (bare_struct_name(s).to_string(), f.to_string());
                *qual_fams.entry(group.clone()).or_default() |= fam;
                qual_keys.entry(group).or_default().push((idx, key.clone()));
            } else {
                *bare_fams.entry(rest.to_string()).or_default() |= fam;
                bare_keys
                    .entry(rest.to_string())
                    .or_default()
                    .push((idx, key.clone()));
            }
        }
    }

    let mut touched: HashSet<String> = HashSet::new();
    let mut remove: [Vec<String>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for (field, mask) in &bare_fams {
        if mask.count_ones() > 1 {
            touched.insert(field.clone());
            for (idx, key) in &bare_keys[field] {
                remove[*idx].push(key.clone());
            }
        }
    }
    for (group, mask) in &qual_fams {
        if mask.count_ones() > 1 {
            touched.insert(group.1.clone());
            for (idx, key) in &qual_keys[group] {
                remove[*idx].push(key.clone());
            }
        }
    }

    if !touched.is_empty() {
        for (set, keys) in [
            &mut *known_strings,
            &mut *known_floats,
            &mut *known_arrays,
            &mut *known_maps,
        ]
        .into_iter()
        .zip(remove.iter())
        {
            for key in keys {
                set.remove(key);
            }
        }
        for field in &touched {
            known_strings.insert(format!("struct_field_mixed:{}", field));
        }
    }
    touched
}

#[derive(Debug, Clone)]
pub struct ProgramInference {
    pub known_strings: HashSet<String>,
    pub known_floats: HashSet<String>,
    pub known_arrays: HashSet<String>,
    pub known_maps: HashSet<String>,
    pub known_arrays_strict: HashSet<String>,
    pub known_maps_strict: HashSet<String>,
    pub struct_inf: StructInference,
}

impl ProgramInference {
    pub fn analyze(program: &Program) -> Self {
        let (inf, _) = Self::analyze_with_timing(program);
        inf
    }

    pub fn analyze_with_timing(
        program: &Program,
    ) -> (Self, (std::time::Duration, std::time::Duration)) {
        let t_idx = std::time::Instant::now();
        let call_index = crate::codegen::analysis::traversal::CallIndex::build(&program.statements);
        let d_call_index = t_idx.elapsed();

        let t_inf = std::time::Instant::now();
        let mut known_strings =
            collect_known_string_vars_with_index(program, &call_index, &HashSet::new());
        let mut known_floats = collect_known_float_vars_with_index(program, &call_index);
        let mut known_arrays = collect_known_array_vars_with_index(program, &call_index);
        let mut known_maps = collect_known_map_vars_with_index(program, &call_index);
        // Cross-family contradictions (alya-lang/alya#132): fields whose
        // markers span two or more kind families lose every struct-field
        // marker here, and the string pass re-runs with their sentinels
        // present so stale derivations (`fn_param_str`, `fn_ret_str`,
        // per-variable field keys) cannot regenerate. Skipped entirely
        // when nothing contradicts, so coherent programs pay nothing.
        let contradicted = suppress_contradictory_struct_field_markers_inference(
            &mut known_strings,
            &mut known_floats,
            &mut known_arrays,
            &mut known_maps,
        );
        if !contradicted.is_empty() {
            known_strings =
                collect_known_string_vars_with_index(program, &call_index, &contradicted);
        }
        let known_arrays_strict = collect_known_array_vars_strict_with_index(program, &call_index);
        let known_maps_strict = collect_known_map_vars_strict_with_index(program, &call_index);
        let struct_inf = StructInference::analyze(program);
        let d_inference = t_inf.elapsed();

        (
            Self {
                known_strings,
                known_floats,
                known_arrays,
                known_maps,
                known_arrays_strict,
                known_maps_strict,
                struct_inf,
            },
            (d_call_index, d_inference),
        )
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
    pub fn infer_param_is_array_strict(
        &self,
        func_name: &str,
        param_idx: usize,
        program: &Program,
    ) -> bool {
        infer_param_is_array_strict_with(func_name, param_idx, program, &self.known_arrays_strict)
    }

    #[inline]
    pub fn infer_param_is_map_strict(
        &self,
        func_name: &str,
        param_idx: usize,
        program: &Program,
    ) -> bool {
        infer_param_is_map_strict_with(func_name, param_idx, program, &self.known_maps_strict)
    }

    #[inline]
    pub fn infer_param_struct_type(&self, func_name: &str, param_idx: usize) -> Option<String> {
        let bare = resolve_func_bare(func_name, &self.struct_inf.struct_names);
        if self
            .struct_inf
            .conflicted_params
            .contains(&(func_name.to_string(), param_idx))
            || self
                .struct_inf
                .conflicted_params
                .contains(&(bare.to_string(), param_idx))
        {
            return None;
        }
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
        let bare = resolve_func_bare(func_name, &self.struct_inf.struct_names);
        self.struct_inf
            .fn_returns
            .get(func_name)
            .or_else(|| self.struct_inf.fn_returns.get(bare))
            .cloned()
    }
}
