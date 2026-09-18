use super::expr::Expr;

#[derive(Debug, Clone, PartialEq)]
pub struct ExternParam {
    pub name: String,
    pub param_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternFnDecl {
    pub name: String,
    pub params: Vec<ExternParam>,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportSymbol {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Import {
        path: String,
        alias: Option<String>,
        symbols: Option<Vec<ImportSymbol>>,
    },
    ExternBlock {
        abi: String,
        lib: Option<String>,
        functions: Vec<ExternFnDecl>,
    },
    Say(Expr),
    Let {
        name: String,
        type_ann: Option<String>,
        value: Expr,
    },
    Const {
        name: String,
        value: Expr,
    },
    Assign {
        name: String,
        value: Expr,
    },
    IndexAssign {
        array: Expr,
        index: Expr,
        value: Expr,
    },
    FieldAssign {
        object: Expr,
        field: String,
        value: Expr,
    },
    StructDef {
        name: String,
        fields: Vec<String>,
        field_types: Vec<Option<String>>,
        defaults: Vec<Option<Expr>>,
    },
    EnumDef {
        name: String,
        variants: Vec<(String, Option<Expr>)>,
    },
    If {
        condition: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    Repeat {
        body: Vec<Stmt>,
    },
    For {
        var: String,
        start: Expr,
        end: Expr,
        body: Vec<Stmt>,
    },
    ForEach {
        var: String,
        value_var: Option<String>,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    Function {
        name: String,
        params: Vec<String>,
        param_types: Vec<Option<String>>,
        return_type: Option<String>,
        defaults: Vec<Option<Expr>>,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    Expr(Expr),
    Throw(Option<Expr>),
    TryCatch {
        try_block: Vec<Stmt>,
        catch_var: Option<String>,
        catch_block: Vec<Stmt>,
        finally_block: Option<Vec<Stmt>>,
    },
    Defer(Box<Stmt>),
    Pub(Box<Stmt>),
    InterfaceDef {
        name: String,
        methods: Vec<InterfaceMethod>,
        embedded: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceMethod {
    pub name: String,
    pub params: Vec<String>,
    pub param_types: Vec<Option<String>>,
    pub return_type: Option<String>,
}

impl Stmt {
    pub fn is_pub(&self) -> bool {
        matches!(self, Stmt::Pub(_))
    }

    pub fn inner_stmt(&self) -> &Stmt {
        match self {
            Stmt::Pub(inner) => inner.inner_stmt(),
            other => other,
        }
    }

    pub fn inner_stmt_mut(&mut self) -> &mut Stmt {
        match self {
            Stmt::Pub(inner) => inner.inner_stmt_mut(),
            other => other,
        }
    }

    pub fn declared_symbol_name(&self) -> Option<&str> {
        match self.inner_stmt() {
            Stmt::Function { name, .. } => Some(name),
            Stmt::StructDef { name, .. } => Some(name),
            Stmt::EnumDef { name, .. } => Some(name),
            Stmt::InterfaceDef { name, .. } => Some(name),
            Stmt::Const { name, .. } => Some(name),
            Stmt::Let { name, .. } => Some(name),
            _ => None,
        }
    }
}
