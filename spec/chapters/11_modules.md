# Chapter 11: Modules, Packages & Visibility

## 1. Specification Rules

### 1.1 Overview
Alya features a file-system aligned module system. Each `.alya` file is an isolated compilation module and namespace. Circular dependencies are resolved at compile time or flagged as illegal recursion.

### 1.2 Visibility Semantics (`pub`)
- By default, all top-level symbols (functions, structs, enums, constants, variables) are **private** to their defining file.
- The `pub` modifier exports a symbol, making it accessible to external modules:
  ```alya
  pub const API_VERSION = "2.0"

  pub struct ServerConfig
      host: string
      port: int
  end

  pub function start_server(cfg: ServerConfig)
      # ...
  end
  ```

### 1.3 Module Import Formats
Modules can be imported via three canonical styles:

#### 1. Whole Module Import (Namespaced)
Imports the entire module under its filename basename:
```alya
import "std/math"

let root = math.sqrt(16.0)
```

#### 2. Aliased Module Import
Renames the imported module namespace to avoid naming collisions:
```alya
import "std/crypto/sha256" as sha

let hash = sha.digest("secret")
```

#### 3. Selective Symbol Import (`from ... import`)
Imports specific public symbols directly into the local scope:
```alya
from "std/math" import sqrt, PI, sin
from "./geometry" import Point2D, calculate_area
```

Selective imports can also alias individual symbols:
```alya
from "std/collections" import Map as HashMap, Set
```

### 1.4 Module Resolution Rules
1. **Standard Library:** Paths beginning with `"std/"` resolve to the bundled Alya standard library directory.
2. **Relative Paths:** Paths beginning with `"./"` or `"../"` resolve relative to the current file's directory.
3. **Third-Party / Packages:** Bare names or `"pkg/..."` resolve against the project's dependency manifest (`alya.toml`). Under strict dependency isolation, transitive dependencies are not accessible without explicit declaration in the active project manifest.
4. **Multi-Major Package Scoping:** When multiple major versions of an external package coexist across dependencies (e.g. `z-v1` and `z-v2`), import statements in each module bind strictly to the major version declared in that module's owning package manifest.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
ImportStmt    ::= "import" StringLit ( "as" Ident )?
                | "from" ( StringLit | ModulePath ) "import" ImportSymbolList

ModulePath    ::= Ident ( ( "::" | "/" ) Ident )*

ImportSymbolList ::= ImportSymbol ( "," ImportSymbol )*
ImportSymbol  ::= Ident ( "as" Ident )?

PubDecl       ::= "pub" ( FunctionDecl | StructDef | EnumDef | ConstDecl | VarDecl )
```
