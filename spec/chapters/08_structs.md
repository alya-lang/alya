# Chapter 08: Structs & Methods (Object Model)

## 1. Specification Rules

### 1.1 Philosophy: Pure Data + UFCS
Alya deliberately avoids classical classes, inheritance hierarchies, and virtual method tables. Instead, it embraces the **Pure Data Struct** model paired with **Uniform Function Call Syntax (UFCS)**:
- Structs are strictly data containers with defined memory layouts.
- Behavior is attached via functions in the struct's namespace.
- No `class`, `extends`, `super`, or polymorphism runtime overhead.
- Composition is preferred over inheritance.

### 1.2 Struct Declaration
A struct is declared with the `struct` keyword and ended with `end`:
```alya
struct User
    id: int
    username: string
    email: string
    is_active: bool = true
end
```
- Fields have explicit types (`field: Type`).
- Fields can specify default initial values (`field: Type = default_value`).
- Fields without default values are required during instantiation.

### 1.3 Instantiation & Field Access
Structs are instantiated using brace syntax:
```alya
let user = User {
    id: 101,
    username: "alicew",
    email: "alice@example.com"
}
```
- **Field Defaults:** Fields with default values can be omitted during instantiation.
- **Field Shorthand (Punning):** When a variable matches the field name, `User { id, username, email }` can be written instead of `User { id: id, ... }`.
- **Field Access & Mutation:** Accessed with dot syntax `user.id`. Modifiable with `user.is_active = false`.

### 1.4 Instance Methods
An instance method is a function whose first parameter is `self`:
```alya
function User.display_name(self) -> string
    return f"{self.username} (ID: {self.id})"
end
```
- Invocation uses standard dot syntax: `user.display_name()`.
- Under the hood, this translates cleanly to `User.display_name(user)`.

### 1.5 Static / Associated Methods
Functions declared under the struct namespace without `self` are static/associated functions (commonly used as factories/constructors):
```alya
function User.create(id: int, username: string, email: string) -> User
    return User {
        id: id,
        username: username,
        email: email,
        is_active: true
    }
end

let new_user = User.create(102, "bob", "bob@example.com")
```

### 1.6 Composition
Complex hierarchies are modeled cleanly by embedding structs:
```alya
struct Address
    city: string
    zip_code: string
end

struct Customer
    name: string
    address: Address
end

let c = Customer {
    name: "Acme Corp",
    address: Address {
        city: "Istanbul",
        zip_code: "34000"
    }
}
say c.address.city
```

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
StructDef     ::= "struct" Ident StructFieldList "end"

StructFieldList ::= ( StructField )*
StructField   ::= Ident ":" Type ( "=" Expr )?

StructInit    ::= Ident "{" ( FieldInitList )? "}"
FieldInitList ::= FieldInit ( "," FieldInit )*
FieldInit     ::= Ident ( ":" Expr )?
```
