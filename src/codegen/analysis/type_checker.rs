use crate::ast::{BinaryOp, Expr, Program, Stmt, UnaryOp};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Returns true for bare generic type parameter names (`T`, `K`, `V`, `TKey`).
/// Convention: starts with an ASCII uppercase letter and contains only
/// alphanumeric/underscore characters while not matching any concrete type
/// name spelling used by the language (those are lowercase or SIMD names).
fn is_bare_type_param(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() => {}
        _ => return false,
    }
    if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return false;
    }
    // Exclude concrete nominal types (PascalCase with lowercase tail).
    !s.chars().skip(1).any(|c| c.is_ascii_lowercase())
}

/// Represents types within Alya's static gradual type system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Any,
    Int,
    I32,
    I16,
    I8,
    UInt,
    U32,
    U16,
    U8,
    Float,
    F32,
    Bool,
    String,
    Rune,
    Null,
    Void,
    Ptr,
    F64x4,
    F32x8,
    I32x8,
    I64x4,
    Array(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Struct(String),
    Enum(String),
    Nullable(Box<Type>),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Any => write!(f, "any"),
            Type::Int => write!(f, "int"),
            Type::I32 => write!(f, "i32"),
            Type::I16 => write!(f, "i16"),
            Type::I8 => write!(f, "i8"),
            Type::UInt => write!(f, "uint"),
            Type::U32 => write!(f, "u32"),
            Type::U16 => write!(f, "u16"),
            Type::U8 => write!(f, "u8"),
            Type::Float => write!(f, "float"),
            Type::F32 => write!(f, "f32"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Rune => write!(f, "rune"),
            Type::Null => write!(f, "null"),
            Type::Void => write!(f, "void"),
            Type::Ptr => write!(f, "ptr"),
            Type::F64x4 => write!(f, "f64x4"),
            Type::F32x8 => write!(f, "f32x8"),
            Type::I32x8 => write!(f, "i32x8"),
            Type::I64x4 => write!(f, "i64x4"),
            Type::Array(elem) => write!(f, "{}[]", elem),
            Type::Map(k, v) => write!(f, "map[{}, {}]", k, v),
            Type::Tuple(elems) => {
                write!(f, "(")?;
                for (i, el) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", el)?;
                }
                write!(f, ")")
            }
            Type::Struct(name) => write!(f, "{}", name),
            Type::Enum(name) => write!(f, "{}", name),
            Type::Nullable(inner) => write!(f, "{}?", inner),
        }
    }
}

impl Type {
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Type::Int
                | Type::I32
                | Type::I16
                | Type::I8
                | Type::UInt
                | Type::U32
                | Type::U16
                | Type::U8
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Type::Float | Type::F32)
    }

    pub fn is_vector(&self) -> bool {
        matches!(self, Type::F64x4 | Type::F32x8 | Type::I32x8 | Type::I64x4)
    }

    /// Determines whether a value of type `self` can be assigned to a location expecting `target`.
    /// Under gradual typing principles:
    /// - `Type::Any` is bidirectionally compatible with all types.
    /// - Explicit annotations are statically verified.
    pub fn is_assignable_to(&self, target: &Type) -> bool {
        // Gradual typing: Any is assignable to any type, and any type is assignable to Any
        if matches!(self, Type::Any) || matches!(target, Type::Any) {
            return true;
        }

        if self == target {
            return true;
        }

        // Nullable handling
        if matches!(self, Type::Null) {
            return matches!(target, Type::Nullable(_) | Type::Ptr);
        }
        if let Type::Nullable(target_inner) = target {
            return self.is_assignable_to(target_inner);
        }

        // Integer subtyping
        if self.is_integer() && target.is_integer() {
            return true;
        }

        // Float subtyping
        if self.is_float() && target.is_float() {
            return true;
        }

        // Generic erasure: codegen monomorphizes by erasing type parameters,
        // so the checker must accept the erased forms too.
        // - A bare type parameter (`T`, `K`, `V`, ...) accepts any concrete
        //   type and vice versa (substitution happens at monomorphization).
        // - `Foo[...]` and `Foo` share a base name and are equivalent here.
        if matches!(self, Type::Struct(_)) || matches!(target, Type::Struct(_)) {
            let self_s = match self {
                Type::Struct(s) => Some(s.as_str()),
                _ => None,
            };
            let target_s = match target {
                Type::Struct(s) => Some(s.as_str()),
                _ => None,
            };
            if self_s.is_some_and(is_bare_type_param) || target_s.is_some_and(is_bare_type_param) {
                return true;
            }
            if let (Some(a), Some(b)) = (self_s, target_s) {
                let base_a = a.split('[').next().unwrap_or(a);
                let base_b = b.split('[').next().unwrap_or(b);
                if base_a == base_b {
                    return true;
                }
            }
        }

        // Vector subtyping and struct equivalence
        match (self, target) {
            (Type::F64x4, Type::Struct(s)) | (Type::Struct(s), Type::F64x4) if s == "f64x4" => {
                return true
            }
            (Type::F32x8, Type::Struct(s)) | (Type::Struct(s), Type::F32x8) if s == "f32x8" => {
                return true
            }
            (Type::I32x8, Type::Struct(s)) | (Type::Struct(s), Type::I32x8) if s == "i32x8" => {
                return true
            }
            (Type::I64x4, Type::Struct(s)) | (Type::Struct(s), Type::I64x4) if s == "i64x4" => {
                return true
            }
            _ => {}
        }

        // String subtyping
        if matches!(self, Type::String) && matches!(target, Type::String) {
            return true;
        }

        // Byte strings share the runtime C-string representation, so a string
        // value is accepted where `u8[]` is annotated (Chapter 00 §1.7).
        // Annotations are check-time only; codegen is unaffected.
        if matches!(self, Type::String) {
            if let Type::Array(elem) = target {
                if matches!(**elem, Type::U8) {
                    return true;
                }
            }
        }

        // Bool subtyping
        if matches!(self, Type::Bool) && matches!(target, Type::Bool) {
            return true;
        }

        // Bool and Int interoperability (booleans are represented as numeric 1/0 in AST)
        if (self.is_integer() && matches!(target, Type::Bool))
            || (matches!(self, Type::Bool) && target.is_integer())
        {
            return true;
        }

        // Rune and Int interoperability (runes are represented as numeric codepoints in AST)
        if (self.is_integer() && matches!(target, Type::Rune))
            || (matches!(self, Type::Rune) && target.is_integer())
        {
            return true;
        }

        // Rune subtyping
        if matches!(self, Type::Rune) && matches!(target, Type::Rune) {
            return true;
        }

        // Array subtyping
        if let (Type::Array(src_el), Type::Array(dst_el)) = (self, target) {
            return src_el.is_assignable_to(dst_el);
        }

        // Map subtyping
        if let (Type::Map(sk, sv), Type::Map(dk, dv)) = (self, target) {
            return sk.is_assignable_to(dk) && sv.is_assignable_to(dv);
        }

        // Tuple subtyping
        if let (Type::Tuple(src_els), Type::Tuple(dst_els)) = (self, target) {
            if src_els.len() == dst_els.len() {
                return src_els
                    .iter()
                    .zip(dst_els.iter())
                    .all(|(s, d)| s.is_assignable_to(d));
            }
        }

        // Array to Tuple assignability (tuples are represented as arrays in AST)
        if let (Type::Array(src_elem), Type::Tuple(dst_parts)) = (self, target) {
            return matches!(**src_elem, Type::Any)
                || dst_parts.iter().all(|p| src_elem.is_assignable_to(p));
        }

        // Struct nominal subtyping
        if let (Type::Struct(s1), Type::Struct(s2)) = (self, target) {
            let b1 = s1.rsplit("::").next().unwrap_or(s1);
            let b1 = b1.rsplit("__").next().unwrap_or(b1);
            let b2 = s2.rsplit("::").next().unwrap_or(s2);
            let b2 = b2.rsplit("__").next().unwrap_or(b2);
            return b1 == b2;
        }

        // Enum nominal subtyping
        if let (Type::Enum(e1), Type::Enum(e2)) = (self, target) {
            let b1 = e1.rsplit("::").next().unwrap_or(e1);
            let b2 = e2.rsplit("::").next().unwrap_or(e2);
            return b1 == b2;
        }

        // Enums and integers or strings are interoperable
        if ((matches!(self, Type::Int) || matches!(self, Type::String))
            && matches!(target, Type::Enum(_)))
            || (matches!(self, Type::Enum(_))
                && (matches!(target, Type::Int) || matches!(target, Type::String)))
        {
            return true;
        }

        // Pointer
        if matches!(self, Type::Ptr) && matches!(target, Type::Ptr) {
            return true;
        }

        false
    }
}

fn split_type_params(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut current = String::new();

    for c in s.chars() {
        match c {
            '(' | '[' | '{' | '<' => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' | '}' | '>' => {
                if depth > 0 {
                    depth -= 1;
                }
                current.push(c);
            }
            ',' if depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(c);
            }
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

/// Parses a string representation of a type into a `Type` enum.
pub fn parse_type_str(raw: &str) -> Type {
    let mut s = raw.trim();
    if s.is_empty() {
        return Type::Any;
    }

    // Weak `weak T` is a null-or-T reference (Chapter 16 §1.4: a weak
    // reference safely becomes null when its target is deallocated), so it
    // parses to Nullable rather than dropping the marker.
    if let Some(rest) = s.strip_prefix("weak ") {
        let inner = parse_type_str(rest.trim());
        return Type::Nullable(Box::new(inner));
    }
    if let Some(rest) = s.strip_prefix("...") {
        s = rest.trim();
    }

    // Nullable T?
    if let Some(stripped) = s.strip_suffix('?') {
        let inner = parse_type_str(stripped);
        return Type::Nullable(Box::new(inner));
    }

    // Array T[]
    if let Some(stripped) = s.strip_suffix("[]") {
        let inner = parse_type_str(stripped);
        return Type::Array(Box::new(inner));
    }

    // Tuple (T1, T2, ...)
    if s.starts_with('(') && s.ends_with(')') {
        let inner = &s[1..s.len() - 1];
        let parts = split_type_params(inner)
            .into_iter()
            .map(|p| parse_type_str(&p))
            .collect();
        return Type::Tuple(parts);
    }

    // Map [K: V] or map[K, V]
    if (s.starts_with('[') && s.ends_with(']')) || (s.starts_with("map[") && s.ends_with(']')) {
        let content = if s.starts_with("map[") {
            &s[4..s.len() - 1]
        } else {
            &s[1..s.len() - 1]
        };
        if let Some((k, v)) = content.split_once(':') {
            return Type::Map(Box::new(parse_type_str(k)), Box::new(parse_type_str(v)));
        } else if let Some((k, v)) = content.split_once(',') {
            return Type::Map(Box::new(parse_type_str(k)), Box::new(parse_type_str(v)));
        } else {
            return Type::Array(Box::new(parse_type_str(content)));
        }
    }

    // Generic array[T]
    if s.starts_with("array[") && s.ends_with(']') {
        let inner = &s[6..s.len() - 1];
        return Type::Array(Box::new(parse_type_str(inner)));
    }

    match s {
        "any" | "auto" => Type::Any,
        "int" | "i64" | "isize" => Type::Int,
        "i32" => Type::I32,
        "i16" => Type::I16,
        "i8" => Type::I8,
        "uint" | "u64" | "usize" => Type::UInt,
        "u32" => Type::U32,
        "u16" => Type::U16,
        "u8" | "byte" => Type::U8,
        "float" | "f64" => Type::Float,
        "f32" => Type::F32,
        "bool" | "boolean" => Type::Bool,
        "string" | "str" => Type::String,
        "rune" | "char" => Type::Rune,
        "null" => Type::Null,
        "void" => Type::Void,
        "ptr" => Type::Ptr,
        "f64x4" => Type::F64x4,
        "f32x8" => Type::F32x8,
        "i32x8" => Type::I32x8,
        "i64x4" => Type::I64x4,
        "array" => Type::Array(Box::new(Type::Any)),
        "map" => Type::Map(Box::new(Type::Any), Box::new(Type::Any)),
        other => Type::Struct(other.to_string()),
    }
}

#[derive(Debug, Clone)]
struct FnSig {
    #[allow(dead_code)]
    name: String,
    param_types: Vec<Type>,
    return_type: Type,
}

#[derive(Debug, Clone)]
struct StructSig {
    #[allow(dead_code)]
    name: String,
    fields: HashMap<String, Type>,
    positional_fields: Vec<Type>,
}

pub struct TypeChecker {
    scopes: Vec<HashMap<String, Type>>,
    assigned: Vec<HashSet<String>>,
    functions: HashMap<String, FnSig>,
    structs: HashMap<String, StructSig>,
    enums: HashSet<String>,
    interfaces: HashMap<String, Vec<String>>,
    struct_methods: HashMap<String, HashSet<String>>,
    /// Deprecated functions: name -> (message, is_error). Collected from
    /// `@deprecated` attributes; enforced at call sites.
    deprecated: HashMap<String, (String, bool)>,
    /// Non-fatal diagnostics (e.g. deprecation warnings) accumulated during
    /// checking. The driver prints these; `validate_types` discards them.
    warnings: Vec<String>,
    current_fn_return_type: Option<Type>,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut tc = Self {
            scopes: vec![HashMap::new()],
            assigned: vec![HashSet::new()],
            functions: HashMap::new(),
            structs: HashMap::new(),
            enums: HashSet::new(),
            interfaces: HashMap::new(),
            struct_methods: HashMap::new(),
            deprecated: HashMap::new(),
            warnings: Vec::new(),
            current_fn_return_type: None,
        };
        tc.register_builtins();
        tc
    }

    fn register_builtins(&mut self) {
        // Built-in core functions
        self.functions.insert(
            "str".to_string(),
            FnSig {
                name: "str".to_string(),
                param_types: vec![Type::Any],
                return_type: Type::String,
            },
        );
        self.functions.insert(
            "int".to_string(),
            FnSig {
                name: "int".to_string(),
                param_types: vec![Type::Any],
                return_type: Type::Int,
            },
        );
        self.functions.insert(
            "float".to_string(),
            FnSig {
                name: "float".to_string(),
                param_types: vec![Type::Any],
                return_type: Type::Float,
            },
        );
        self.functions.insert(
            "bool".to_string(),
            FnSig {
                name: "bool".to_string(),
                param_types: vec![Type::Any],
                return_type: Type::Bool,
            },
        );
        self.functions.insert(
            "len".to_string(),
            FnSig {
                name: "len".to_string(),
                param_types: vec![Type::Any],
                return_type: Type::Int,
            },
        );
        self.functions.insert(
            "sqrt".to_string(),
            FnSig {
                name: "sqrt".to_string(),
                param_types: vec![Type::Float],
                return_type: Type::Float,
            },
        );

        // --- SIMD Vector Built-in Functions ---
        self.functions.insert(
            "simd_f64x4_new".to_string(),
            FnSig {
                name: "simd_f64x4_new".to_string(),
                param_types: vec![Type::Float, Type::Float, Type::Float, Type::Float],
                return_type: Type::Ptr,
            },
        );
        self.functions.insert(
            "simd_f64x4_splat".to_string(),
            FnSig {
                name: "simd_f64x4_splat".to_string(),
                param_types: vec![Type::Float],
                return_type: Type::Ptr,
            },
        );
        self.functions.insert(
            "simd_f64x4_add".to_string(),
            FnSig {
                name: "simd_f64x4_add".to_string(),
                param_types: vec![Type::Ptr, Type::Ptr],
                return_type: Type::Ptr,
            },
        );
        self.functions.insert(
            "simd_f64x4_sub".to_string(),
            FnSig {
                name: "simd_f64x4_sub".to_string(),
                param_types: vec![Type::Ptr, Type::Ptr],
                return_type: Type::Ptr,
            },
        );
        self.functions.insert(
            "simd_f64x4_mul".to_string(),
            FnSig {
                name: "simd_f64x4_mul".to_string(),
                param_types: vec![Type::Ptr, Type::Ptr],
                return_type: Type::Ptr,
            },
        );
        self.functions.insert(
            "simd_f64x4_div".to_string(),
            FnSig {
                name: "simd_f64x4_div".to_string(),
                param_types: vec![Type::Ptr, Type::Ptr],
                return_type: Type::Ptr,
            },
        );
        self.functions.insert(
            "simd_f64x4_fma".to_string(),
            FnSig {
                name: "simd_f64x4_fma".to_string(),
                param_types: vec![Type::Ptr, Type::Ptr, Type::Ptr],
                return_type: Type::Ptr,
            },
        );
        self.functions.insert(
            "simd_f64x4_sum".to_string(),
            FnSig {
                name: "simd_f64x4_sum".to_string(),
                param_types: vec![Type::Ptr],
                return_type: Type::Float,
            },
        );
        self.functions.insert(
            "simd_f64x4_min".to_string(),
            FnSig {
                name: "simd_f64x4_min".to_string(),
                param_types: vec![Type::Ptr],
                return_type: Type::Float,
            },
        );
        self.functions.insert(
            "simd_f64x4_max".to_string(),
            FnSig {
                name: "simd_f64x4_max".to_string(),
                param_types: vec![Type::Ptr],
                return_type: Type::Float,
            },
        );
        self.functions.insert(
            "simd_f64x4_get".to_string(),
            FnSig {
                name: "simd_f64x4_get".to_string(),
                param_types: vec![Type::Ptr, Type::Int],
                return_type: Type::Float,
            },
        );
        self.functions.insert(
            "simd_f64x4_set".to_string(),
            FnSig {
                name: "simd_f64x4_set".to_string(),
                param_types: vec![Type::Ptr, Type::Int, Type::Float],
                return_type: Type::Ptr,
            },
        );
        self.functions.insert(
            "simd_f64x4_load".to_string(),
            FnSig {
                name: "simd_f64x4_load".to_string(),
                param_types: vec![Type::Ptr, Type::Int],
                return_type: Type::Ptr,
            },
        );
        self.functions.insert(
            "simd_f64x4_store".to_string(),
            FnSig {
                name: "simd_f64x4_store".to_string(),
                param_types: vec![Type::Ptr, Type::Int, Type::Ptr],
                return_type: Type::Void,
            },
        );
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.assigned.push(HashSet::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
        self.assigned.pop();
    }

    fn define_var(&mut self, name: &str, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty.clone());
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if bare != name {
                scope.insert(bare.to_string(), ty);
            }
        }
        self.mark_assigned(name);
    }

    /// Marks a variable as definitely assigned (any `Assign` to it counts,
    /// flow-insensitively: assignment inside any branch satisfies later reads).
    /// The mark lands on the scope where the variable is declared, so
    /// branch-level assignments survive scope pops.
    fn mark_assigned(&mut self, name: &str) {
        let bare = name.rsplit("::").next().unwrap_or(name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        let mut target_idx: Option<usize> = None;
        for (idx, scope) in self.scopes.iter().enumerate().rev() {
            if scope.contains_key(name) || scope.contains_key(bare) {
                target_idx = Some(idx);
                break;
            }
        }
        let idx = target_idx.unwrap_or_else(|| self.assigned.len().saturating_sub(1));
        if let Some(set) = self.assigned.get_mut(idx) {
            set.insert(name.to_string());
            if bare != name {
                set.insert(bare.to_string());
            }
        }
    }

    /// Marks a variable as declared-but-unassigned (deferred `let x: T`).
    fn mark_unassigned(&mut self, name: &str) {
        let bare = name.rsplit("::").next().unwrap_or(name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        for set in self.assigned.iter_mut().rev() {
            set.remove(name);
            set.remove(bare);
        }
    }

    /// Definite-assignment read check: a declared-but-unassigned variable
    /// must not be read (Chapter 01 §1.1). Unknown names stay lenient.
    fn is_assigned(&self, name: &str) -> bool {
        let bare = name.rsplit("::").next().unwrap_or(name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        for set in self.assigned.iter().rev() {
            if set.contains(name) || set.contains(bare) {
                return true;
            }
        }
        // Not tracked as assigned: only an error if the name is a declared
        // variable (unknown identifiers keep today's lenient `Any` behavior).
        let mut declared = false;
        for scope in self.scopes.iter().rev() {
            if scope.contains_key(name) || scope.contains_key(bare) {
                declared = true;
                break;
            }
        }
        !declared
    }

    fn resolve_type(&self, ty: Type) -> Type {
        match ty {
            Type::Struct(ref name) => {
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if self.enums.contains(name) || self.enums.contains(bare) {
                    Type::Enum(name.clone())
                } else {
                    ty
                }
            }
            Type::Nullable(inner) => Type::Nullable(Box::new(self.resolve_type(*inner))),
            Type::Array(inner) => Type::Array(Box::new(self.resolve_type(*inner))),
            _ => ty,
        }
    }

    fn resolve_type_str(&self, raw: &str) -> Type {
        self.resolve_type(parse_type_str(raw))
    }

    fn lookup_var(&self, name: &str) -> Option<Type> {
        let bare = name.rsplit("::").next().unwrap_or(name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Some(t.clone());
            }
            if let Some(t) = scope.get(bare) {
                return Some(t.clone());
            }
        }
        None
    }

    fn lookup_fn(&self, name: &str, first_arg_type: Option<&Type>) -> Option<FnSig> {
        // Method lookup via UFCS: receiver.method(...) -> Struct__method(receiver, ...)
        if let Some(fat) = first_arg_type {
            let sname_opt = match fat {
                Type::Struct(s) => Some(s.as_str()),
                Type::F64x4 => Some("f64x4"),
                Type::F32x8 => Some("f32x8"),
                Type::I32x8 => Some("i32x8"),
                Type::I64x4 => Some("i64x4"),
                _ => None,
            };
            if let Some(sname) = sname_opt {
                let bare_struct = sname.rsplit("::").next().unwrap_or(sname);
                let bare_struct = bare_struct.rsplit("__").next().unwrap_or(bare_struct);
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);

                let candidates = [
                    format!("{}__{}", sname, name),
                    format!("{}__{}", bare_struct, name),
                    format!("{}__{}", sname, bare),
                    format!("{}__{}", bare_struct, bare),
                    format!("{}.{}", sname, name),
                    format!("{}.{}", bare_struct, name),
                    format!("{}.{}", sname, bare),
                    format!("{}.{}", bare_struct, bare),
                ];
                for cand in &candidates {
                    if let Some(sig) = self.functions.get(cand) {
                        return Some(sig.clone());
                    }
                }
            }
        }

        if let Some(sig) = self.functions.get(name) {
            return Some(sig.clone());
        }

        let bare = name.rsplit("::").next().unwrap_or(name);
        if let Some(sig) = self.functions.get(bare) {
            return Some(sig.clone());
        }

        None
    }

    /// Infers the static type of an expression.
    pub fn infer_expr(&self, expr: &Expr) -> Result<Type, String> {
        match expr {
            Expr::Number(_) => Ok(Type::Int),
            Expr::Float(_) => Ok(Type::Float),
            Expr::String(_) => Ok(Type::String),
            Expr::Null => Ok(Type::Null),
            Expr::InterpolatedString(_) => Ok(Type::String),
            // Force unwrap strips one nullability layer (Chapter 19 §5).
            Expr::ForceUnwrap(inner) => match self.infer_expr(inner)? {
                Type::Nullable(boxed) => Ok(*boxed),
                other => Ok(other),
            },

            Expr::Identifier(name) => {
                if let Some(ty) = self.lookup_var(name) {
                    if !self.is_assigned(name) {
                        return Err(format!(
                            "TypeError: Variable '{}' is used before assignment",
                            name
                        ));
                    }
                    return Ok(ty);
                }
                if self.enums.contains(name) {
                    return Ok(Type::Enum(name.clone()));
                }
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if self.enums.contains(bare) {
                    return Ok(Type::Enum(bare.to_string()));
                }
                Ok(Type::Any)
            }

            Expr::Array(items) => {
                if items.is_empty() {
                    Ok(Type::Array(Box::new(Type::Any)))
                } else {
                    let first_type = self.infer_expr(&items[0])?;
                    let all_same = items.iter().all(|it| {
                        self.infer_expr(it)
                            .map(|t| t == first_type)
                            .unwrap_or(false)
                    });
                    if all_same && first_type != Type::Any {
                        Ok(Type::Array(Box::new(first_type)))
                    } else {
                        Ok(Type::Array(Box::new(Type::Any)))
                    }
                }
            }

            Expr::Map(pairs) => {
                if pairs.is_empty() {
                    Ok(Type::Map(Box::new(Type::Any), Box::new(Type::Any)))
                } else {
                    let k_type = self.infer_expr(&pairs[0].0)?;
                    let v_type = self.infer_expr(&pairs[0].1)?;
                    Ok(Type::Map(Box::new(k_type), Box::new(v_type)))
                }
            }

            Expr::Binary { op, left, right } => {
                let lt = self.infer_expr(left)?;
                let rt = self.infer_expr(right)?;

                match op {
                    BinaryOp::Add => {
                        if lt == Type::String || rt == Type::String {
                            Ok(Type::String)
                        } else if lt.is_float() || rt.is_float() {
                            Ok(Type::Float)
                        } else if lt.is_integer() && rt.is_integer() {
                            Ok(Type::Int)
                        } else {
                            Ok(Type::Any)
                        }
                    }
                    BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Modulo => {
                        if lt.is_float() || rt.is_float() {
                            Ok(Type::Float)
                        } else if lt.is_integer() && rt.is_integer() {
                            Ok(Type::Int)
                        } else {
                            Ok(Type::Any)
                        }
                    }
                    BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr => Ok(Type::Int),
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::Greater
                    | BinaryOp::LessEqual
                    | BinaryOp::GreaterEqual
                    | BinaryOp::In
                    | BinaryOp::NotIn => Ok(Type::Bool),
                    BinaryOp::And | BinaryOp::Or => Ok(Type::Bool),
                    BinaryOp::Range | BinaryOp::RangeInclusive => {
                        Ok(Type::Array(Box::new(Type::Int)))
                    }
                }
            }

            Expr::Unary { op, expr } => match op {
                UnaryOp::Not => Ok(Type::Bool),
                UnaryOp::BitNot => Ok(Type::Int),
                UnaryOp::Negate => self.infer_expr(expr),
            },

            Expr::Cast { target, .. } => Ok(parse_type_str(target)),
            Expr::TypeCheck { .. } => Ok(Type::Bool),

            Expr::Ternary {
                then_branch,
                else_branch,
                ..
            } => {
                let tt = self.infer_expr(then_branch)?;
                let et = self.infer_expr(else_branch)?;
                if tt == et {
                    Ok(tt)
                } else {
                    Ok(Type::Any)
                }
            }

            Expr::NullCoalesce { value, default } => {
                let vt = self.infer_expr(value)?;
                let dt = self.infer_expr(default)?;
                if let Type::Nullable(inner) = vt {
                    Ok(*inner)
                } else if vt != Type::Null {
                    Ok(vt)
                } else {
                    Ok(dt)
                }
            }

            Expr::StructInit { name, .. } => Ok(Type::Struct(name.clone())),

            Expr::FieldAccess { object, field } | Expr::OptionalFieldAccess { object, field } => {
                let obj_type = self.infer_expr(object)?;
                let base_type = match &obj_type {
                    Type::Nullable(inner) => inner.as_ref(),
                    other => other,
                };
                if let Type::Struct(sname) = base_type {
                    let bare = sname.rsplit("::").next().unwrap_or(sname);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    if let Some(sdef) = self.structs.get(sname).or_else(|| self.structs.get(bare)) {
                        if let Some(ftype) = sdef.fields.get(field) {
                            return Ok(ftype.clone());
                        }
                    }
                }
                if let Type::Tuple(parts) = &obj_type {
                    if let Ok(idx) = field.parse::<usize>() {
                        if let Some(t) = parts.get(idx) {
                            return Ok(t.clone());
                        }
                    }
                }
                if field == "length" || field == "len" {
                    return Ok(Type::Int);
                }
                Ok(Type::Any)
            }

            Expr::Index { array, .. } | Expr::OptionalIndex { array, .. } => {
                let arr_type = self.infer_expr(array)?;
                match arr_type {
                    Type::Array(elem) => Ok(*elem),
                    Type::Map(_, val) => Ok(*val),
                    Type::String => Ok(Type::String),
                    _ => Ok(Type::Any),
                }
            }

            Expr::Call { name, args } | Expr::OptionalCall { callee: name, args } => {
                if let Some(sdef) = self.structs.get(name) {
                    return Ok(Type::Struct(sdef.name.clone()));
                }

                let first_arg_type = args.first().and_then(|a| self.infer_expr(a).ok());
                if let Some(sig) = self.lookup_fn(name, first_arg_type.as_ref()) {
                    return Ok(sig.return_type);
                }

                match name.as_str() {
                    "str" => Ok(Type::String),
                    "int" | "len" => Ok(Type::Int),
                    "float" => Ok(Type::Float),
                    "bool" => Ok(Type::Bool),
                    "rune" | "char" => Ok(Type::Rune),
                    _ => Ok(Type::Any),
                }
            }
        }
    }

    /// Recursively checks an expression and validates all sub-expressions.
    pub fn check_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Identifier(name)
                if self.lookup_var(name).is_some() && !self.is_assigned(name) =>
            {
                // Definite-assignment read check (Chapter 01 §1.1). Unknown
                // names stay lenient here; inference resolves them to `Any`.
                return Err(format!(
                    "TypeError: Variable '{}' is used before assignment",
                    name
                ));
            }
            Expr::Binary { left, right, .. } => {
                self.check_expr(left)?;
                self.check_expr(right)?;
            }
            Expr::Unary { expr, .. } | Expr::Cast { expr, .. } | Expr::TypeCheck { expr, .. } => {
                self.check_expr(expr)?;
            }
            Expr::ForceUnwrap(inner) => {
                self.check_expr(inner)?;
            }
            Expr::Array(items) | Expr::InterpolatedString(items) => {
                for it in items {
                    self.check_expr(it)?;
                }
            }
            Expr::Map(pairs) => {
                for (k, v) in pairs {
                    self.check_expr(k)?;
                    self.check_expr(v)?;
                }
            }
            Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
                self.check_expr(array)?;
                self.check_expr(index)?;
            }
            Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
                self.check_expr(object)?;
            }
            Expr::Ternary {
                condition,
                then_branch,
                else_branch,
            } => {
                self.check_expr(condition)?;
                self.check_expr(then_branch)?;
                self.check_expr(else_branch)?;
            }
            Expr::NullCoalesce { value, default } => {
                self.check_expr(value)?;
                self.check_expr(default)?;
            }
            Expr::StructInit { name, fields } => {
                for (_, val) in fields {
                    self.check_expr(val)?;
                }

                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if let Some(sdef) = self.structs.get(name).or_else(|| self.structs.get(bare)) {
                    for (fname, fval) in fields {
                        if let Some(expected_ft) = sdef.fields.get(fname) {
                            let actual_vt = self.infer_expr(fval)?;
                            if !self.types_compatible(&actual_vt, expected_ft) {
                                return Err(format!(
                                    "TypeError: Type mismatch for field '{}.{}': expected '{}', found '{}'",
                                    name, fname, expected_ft, actual_vt
                                ));
                            }
                        }
                    }
                }
            }
            Expr::Call { name, args } | Expr::OptionalCall { callee: name, args } => {
                for a in args {
                    self.check_expr(a)?;
                }

                self.check_deprecated_call(name)?;

                let first_arg_type = args.first().and_then(|a| self.infer_expr(a).ok());
                if let Some(sig) = self.lookup_fn(name, first_arg_type.as_ref()) {
                    for (i, param_ty) in sig.param_types.iter().enumerate() {
                        if param_ty != &Type::Any {
                            if let Some(arg) = args.get(i) {
                                let arg_ty = self.infer_expr(arg)?;
                                if !self.types_compatible(&arg_ty, param_ty) {
                                    return Err(format!(
                                        "TypeError: Type mismatch for argument {} of function '{}': expected '{}', found '{}'",
                                        i + 1,
                                        name,
                                        param_ty,
                                        arg_ty
                                    ));
                                }
                            }
                        }
                    }
                } else if let Some(sdef) = self.structs.get(name) {
                    for (i, field_ty) in sdef.positional_fields.iter().enumerate() {
                        if field_ty != &Type::Any {
                            if let Some(arg) = args.get(i) {
                                let arg_ty = self.infer_expr(arg)?;
                                if !self.types_compatible(&arg_ty, field_ty) {
                                    return Err(format!(
                                        "TypeError: Type mismatch for positional argument {} of struct '{}': expected '{}', found '{}'",
                                        i + 1,
                                        name,
                                        field_ty,
                                        arg_ty
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Checks a statement and validates all typing rules within it.
    pub fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt.inner_stmt() {
            Stmt::Let {
                name,
                type_ann,
                value,
            } => {
                // Deferred declaration (`let x: T` parses with a Null value):
                // no compatibility check, declared type kept, reads blocked
                // until an assignment lands (Chapter 01 §1.1). This path only
                // applies when null is NOT a valid value for the annotation;
                // `let x: T? = null` is a genuine assignment of null.
                if matches!(value, Expr::Null) && type_ann.is_some() {
                    let expected = self.resolve_type_str(type_ann.as_ref().unwrap());
                    if !Type::Null.is_assignable_to(&expected) {
                        self.define_var(name, expected);
                        self.mark_unassigned(name);
                        return Ok(());
                    }
                }
                self.check_expr(value)?;
                let val_type = self.infer_expr(value)?;

                if let Some(ann_str) = type_ann {
                    let expected = self.resolve_type_str(ann_str);
                    if !self.types_compatible(&val_type, &expected) {
                        return Err(format!(
                            "TypeError: Type mismatch in 'let {}': expected '{}', found '{}'",
                            name, expected, val_type
                        ));
                    }

                    if let Type::Array(elem_ty) = &expected {
                        if let Expr::Array(items) = value {
                            for (idx, it) in items.iter().enumerate() {
                                let it_ty = self.infer_expr(it)?;
                                if !self.types_compatible(&it_ty, elem_ty) {
                                    return Err(format!(
                                        "TypeError: Type mismatch in array element {} of 'let {}': expected '{}', found '{}'",
                                        idx, name, elem_ty, it_ty
                                    ));
                                }
                            }
                        }
                    }
                    self.define_var(name, expected);
                } else {
                    self.define_var(name, val_type);
                }
            }

            Stmt::Const { name, value } => {
                self.check_expr(value)?;
                let val_type = self.infer_expr(value)?;
                self.define_var(name, val_type);
            }

            Stmt::Assign { name, value } => {
                self.check_expr(value)?;
                let val_type = self.infer_expr(value)?;

                if let Some(expected) = self.lookup_var(name) {
                    if expected != Type::Any && !self.types_compatible(&val_type, &expected) {
                        return Err(format!(
                            "TypeError: Cannot assign '{}' to variable '{}' of type '{}'",
                            val_type, name, expected
                        ));
                    }
                }
                self.mark_assigned(name);
            }

            Stmt::FieldAssign {
                object,
                field,
                value,
            } => {
                self.check_expr(object)?;
                self.check_expr(value)?;

                let obj_type = self.infer_expr(object)?;
                let val_type = self.infer_expr(value)?;

                if let Type::Struct(sname) = obj_type {
                    let bare = sname.rsplit("::").next().unwrap_or(&sname);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    if let Some(sdef) = self.structs.get(&sname).or_else(|| self.structs.get(bare))
                    {
                        if let Some(expected_ft) = sdef.fields.get(field) {
                            if !self.types_compatible(&val_type, expected_ft) {
                                return Err(format!(
                                    "TypeError: Type mismatch for field '{}.{}': expected '{}', found '{}'",
                                    sname, field, expected_ft, val_type
                                ));
                            }
                        }
                    }
                }
            }

            Stmt::IndexAssign {
                array,
                index,
                value,
            } => {
                self.check_expr(array)?;
                self.check_expr(index)?;
                self.check_expr(value)?;

                let arr_type = self.infer_expr(array)?;
                let val_type = self.infer_expr(value)?;

                if let Type::Array(elem_type) = arr_type {
                    if *elem_type != Type::Any && !self.types_compatible(&val_type, &elem_type) {
                        return Err(format!(
                            "TypeError: Cannot assign '{}' to array of type '{}'",
                            val_type, elem_type
                        ));
                    }
                }
            }

            Stmt::Function {
                name: _,
                params,
                param_types,
                return_type,
                body,
                ..
            } => {
                let declared_ret = return_type
                    .as_ref()
                    .map(|s| self.resolve_type_str(s))
                    .unwrap_or(Type::Any);

                let prev_ret = self.current_fn_return_type.take();
                self.current_fn_return_type = Some(declared_ret.clone());
                self.push_scope();

                for (pname, ptype_opt) in params.iter().zip(param_types.iter()) {
                    let pty = ptype_opt
                        .as_ref()
                        .map(|s| self.resolve_type_str(s))
                        .unwrap_or(Type::Any);
                    self.define_var(pname, pty);
                }

                for s in body {
                    self.check_stmt(s)?;
                }

                self.pop_scope();
                self.current_fn_return_type = prev_ret;
            }

            Stmt::Return(expr_opt) => {
                let expected_ret = self.current_fn_return_type.clone();
                if let Some(ref expected_ret) = expected_ret {
                    match expr_opt {
                        Some(expr) => {
                            self.check_expr(expr)?;
                            let actual = self.infer_expr(expr)?;
                            if *expected_ret == Type::Void {
                                return Err(
                                    "TypeError: Void function cannot return a value".to_string()
                                );
                            }
                            if expected_ret != &Type::Any
                                && !self.types_compatible(&actual, expected_ret)
                            {
                                return Err(format!(
                                    "TypeError: Return type mismatch: expected '{}', found '{}'",
                                    expected_ret, actual
                                ));
                            }
                        }
                        None => {
                            if expected_ret != &Type::Void && expected_ret != &Type::Any {
                                return Err(format!(
                                    "TypeError: Non-void function must return a value of type '{}'",
                                    expected_ret
                                ));
                            }
                        }
                    }
                } else if let Some(expr) = expr_opt {
                    self.check_expr(expr)?;
                }
            }

            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.check_expr(condition)?;
                self.push_scope();
                for s in then_block {
                    self.check_stmt(s)?;
                }
                self.pop_scope();

                if let Some(eb) = else_block {
                    self.push_scope();
                    for s in eb {
                        self.check_stmt(s)?;
                    }
                    self.pop_scope();
                }
            }

            Stmt::While { condition, body } => {
                self.check_expr(condition)?;
                self.push_scope();
                for s in body {
                    self.check_stmt(s)?;
                }
                self.pop_scope();
            }

            Stmt::Repeat { body } => {
                self.push_scope();
                for s in body {
                    self.check_stmt(s)?;
                }
                self.pop_scope();
            }

            Stmt::For {
                var,
                start,
                end,
                body,
                ..
            } => {
                self.check_expr(start)?;
                self.check_expr(end)?;
                self.push_scope();
                self.define_var(var, Type::Int);
                for s in body {
                    self.check_stmt(s)?;
                }
                self.pop_scope();
            }

            Stmt::ForEach {
                var,
                value_var,
                iterable,
                body,
            } => {
                self.check_expr(iterable)?;
                let iter_type = self.infer_expr(iterable)?;
                self.push_scope();

                match iter_type {
                    Type::Array(elem) => {
                        if let Some(val_name) = value_var {
                            self.define_var(var, Type::Int);
                            self.define_var(val_name, *elem);
                        } else {
                            self.define_var(var, *elem);
                        }
                    }
                    Type::Map(k, v) => {
                        if let Some(val_name) = value_var {
                            self.define_var(var, *k);
                            self.define_var(val_name, *v);
                        } else {
                            self.define_var(var, *k);
                        }
                    }
                    _ => {
                        self.define_var(var, Type::Any);
                        if let Some(val_name) = value_var {
                            self.define_var(val_name, Type::Any);
                        }
                    }
                }

                for s in body {
                    self.check_stmt(s)?;
                }
                self.pop_scope();
            }

            Stmt::TryCatch {
                try_block,
                catch_var,
                catch_block,
                finally_block,
            } => {
                self.push_scope();
                for s in try_block {
                    self.check_stmt(s)?;
                }
                self.pop_scope();

                self.push_scope();
                if let Some(cvar) = catch_var {
                    self.define_var(cvar, Type::String);
                }
                for s in catch_block {
                    self.check_stmt(s)?;
                }
                self.pop_scope();

                if let Some(fb) = finally_block {
                    self.push_scope();
                    for s in fb {
                        self.check_stmt(s)?;
                    }
                    self.pop_scope();
                }
            }

            Stmt::Expr(e) | Stmt::Say(e) => {
                self.check_expr(e)?;
            }

            Stmt::Throw(Some(e)) => {
                self.check_expr(e)?;
            }

            _ => {}
        }
        Ok(())
    }

    /// Structural interface satisfaction: `struct_name` satisfies `iface_name`
    /// when it defines every method the interface requires. Both sides are
    /// compared on bare names (`::`/`__` prefixes stripped).
    fn struct_satisfies_interface(&self, struct_name: &str, iface_name: &str) -> bool {
        let bare_struct = struct_name
            .rsplit("::")
            .next()
            .unwrap_or(struct_name)
            .rsplit("__")
            .next()
            .unwrap_or(struct_name);
        let bare_iface = iface_name
            .rsplit("::")
            .next()
            .unwrap_or(iface_name)
            .rsplit("__")
            .next()
            .unwrap_or(iface_name);
        let required = match self.interfaces.get(bare_iface) {
            Some(methods) => methods,
            None => return false,
        };
        // The interface itself is not a struct value.
        if self.interfaces.contains_key(bare_struct) && bare_struct == bare_iface {
            return true;
        }
        let provided = match self.struct_methods.get(bare_struct) {
            Some(methods) => methods,
            None => return false,
        };
        required.iter().all(|m| provided.contains(m))
    }

    /// Full compatibility check: static assignability OR structural
    /// interface satisfaction (a struct defining all of an interface's
    /// methods is assignable to that interface).
    fn types_compatible(&self, actual: &Type, expected: &Type) -> bool {
        if actual.is_assignable_to(expected) {
            return true;
        }
        let actual_s = match actual {
            Type::Struct(s) => s.as_str(),
            _ => return false,
        };
        let expected_s = match expected {
            Type::Struct(s) => s.as_str(),
            _ => return false,
        };
        self.struct_satisfies_interface(actual_s, expected_s)
    }

    /// Enforces `@deprecated` at a call site. `error = true` is fatal;
    /// otherwise a warning is recorded for the driver to print.
    fn check_deprecated_call(&mut self, name: &str) -> Result<(), String> {
        let bare = name.rsplit("::").next().unwrap_or(name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        let hit = self
            .deprecated
            .get(name)
            .or_else(|| self.deprecated.get(bare))
            .cloned();
        if let Some((message, is_error)) = hit {
            if is_error {
                return Err(format!(
                    "TypeError: call to removed function '{}': {}",
                    name,
                    if message.is_empty() {
                        "removed via @deprecated(error = true)".to_string()
                    } else {
                        message
                    }
                ));
            }
            let detail = if message.is_empty() {
                String::new()
            } else {
                format!(": {}", message)
            };
            self.warnings.push(format!(
                "warning: call to deprecated function '{}'{}",
                name, detail
            ));
        }
        Ok(())
    }

    /// Primary entry point: analyzes and statically type-checks a complete program.
    pub fn check_program(&mut self, program: &Program) -> Result<(), String> {
        // Pass 1: Collect all struct definitions
        for stmt in &program.statements {
            if let Stmt::StructDef {
                name,
                fields,
                field_types,
                ..
            } = stmt.inner_stmt()
            {
                let mut field_map = HashMap::new();
                let mut positional = Vec::new();
                for (fname, ftype_opt) in fields.iter().zip(field_types.iter()) {
                    let ftype = ftype_opt
                        .as_ref()
                        .map(|s| parse_type_str(s))
                        .unwrap_or(Type::Any);
                    field_map.insert(fname.clone(), ftype.clone());
                    positional.push(ftype);
                }

                let sig = StructSig {
                    name: name.clone(),
                    fields: field_map,
                    positional_fields: positional,
                };

                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                self.structs.insert(name.clone(), sig.clone());
                if bare != name {
                    self.structs.insert(bare.to_string(), sig);
                }
            }
        }

        // Pass 2: Collect all enum definitions
        for stmt in &program.statements {
            if let Stmt::EnumDef { name, .. } = stmt.inner_stmt() {
                self.enums.insert(name.clone());
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if bare != name {
                    self.enums.insert(bare.to_string());
                }
            }
        }

        // Pass 2b: Collect interface contracts and struct method sets for
        // structural satisfaction checks.
        for stmt in &program.statements {
            match stmt.inner_stmt() {
                Stmt::InterfaceDef { name, methods, .. } => {
                    let required: Vec<String> = methods.iter().map(|m| m.name.clone()).collect();
                    self.interfaces.insert(name.clone(), required.clone());
                    let bare = name.rsplit("::").next().unwrap_or(name);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    if bare != name {
                        self.interfaces.insert(bare.to_string(), required);
                    }
                }
                Stmt::Function { name, .. } => {
                    // UFCS methods are stored mangled as `Struct__method`.
                    // Only attach when the head is a declared struct, so
                    // generic specializations (`swap__int`) are not mistaken
                    // for methods.
                    if let Some((sname, mname)) = name.split_once("__") {
                        if !sname.is_empty()
                            && !mname.is_empty()
                            && !mname.contains("__")
                            && (self.structs.contains_key(sname)
                                || self
                                    .structs
                                    .keys()
                                    .any(|k| k.rsplit("::").next().unwrap_or(k) == sname))
                        {
                            self.struct_methods
                                .entry(sname.to_string())
                                .or_default()
                                .insert(mname.to_string());
                        }
                    }
                }
                _ => {}
            }
        }

        // Pass 3: Collect all function signatures
        for stmt in &program.statements {
            if let Stmt::Function {
                name,
                params: _,
                param_types,
                return_type,
                attributes,
                ..
            } = stmt.inner_stmt()
            {
                for attr in attributes {
                    if attr.name == "deprecated" {
                        let mut message = String::new();
                        let mut is_error = false;
                        for (key, val) in &attr.args {
                            match key.as_deref() {
                                None if message.is_empty() => message = val.clone(),
                                Some("note") | Some("message") => message = val.clone(),
                                Some("error") => is_error = val == "true" || val == "1",
                                _ => {}
                            }
                        }
                        self.deprecated
                            .insert(name.clone(), (message.clone(), is_error));
                        let bare_mod = name.rsplit("::").next().unwrap_or(name);
                        if bare_mod != name {
                            self.deprecated
                                .insert(bare_mod.to_string(), (message, is_error));
                        }
                    }
                }
                let p_types = param_types
                    .iter()
                    .map(|pt| {
                        pt.as_ref()
                            .map(|s| self.resolve_type_str(s))
                            .unwrap_or(Type::Any)
                    })
                    .collect();

                let r_type = return_type
                    .as_ref()
                    .map(|s| self.resolve_type_str(s))
                    .unwrap_or(Type::Any);

                let sig = FnSig {
                    name: name.clone(),
                    param_types: p_types,
                    return_type: r_type,
                };

                self.functions.insert(name.clone(), sig.clone());

                let bare_mod = name.rsplit("::").next().unwrap_or(name);
                if bare_mod != name && !self.functions.contains_key(bare_mod) {
                    self.functions.insert(bare_mod.to_string(), sig);
                }
            }
        }

        // Pass 4: Check statements and bodies
        for stmt in &program.statements {
            self.check_stmt(stmt)?;
        }

        Ok(())
    }
}

/// Convenience function: validates typing contracts across a program AST.
pub fn validate_types(program: &Program) -> Result<(), String> {
    validate_types_with_warnings(program).1
}

/// Validates typing contracts, returning accumulated non-fatal warnings
/// (e.g. deprecation notices) alongside the result.
pub fn validate_types_with_warnings(program: &Program) -> (Vec<String>, Result<(), String>) {
    let mut checker = TypeChecker::new();
    let result = checker.check_program(program);
    (checker.warnings, result)
}
