# Chapter 4: Functions & Modules

[← Control Flow](control-flow.md) • [Wiki Home](README.md) • [Next: Data Structures →](data-structures.md)

---

## 1. Functions

Functions in Alya are declared using the `function` keyword and closed with `end`. Values are returned using `return`. If execution reaches the end of a function without a `return` statement, `0` is returned implicitly.

```alya
function add(x, y)
    return x + y
end

let sum = add(10, 25)
say sum    # 35
```

### Default Parameter Values
Parameters can specify default fallback values, evaluated when caller omits them:

```alya
function greet(name, greeting = "Hello", punctuation = "!")
    say "{greeting}, {name}{punctuation}"
end

greet("World")                  # "Hello, World!"
greet("Alice", "Welcome")       # "Welcome, Alice!"
greet("Bob", "Good day", "?")   # "Good day, Bob?"
```

### Call Conventions & Native Stack Frames
Alya generates native ABI-compliant function calls (`call` on x64, `bl` on ARM64) with stack frame alignment and standard register calling conventions, avoiding any interpreter dispatch overhead.

### Multiple Return Values & Tuples
Functions can return multiple values separated by commas. Callers can unpack them directly using tuple destructuring:

```alya
function min_max(a, b)
    if a < b
        return a, b
    end
    return b, a
end

let (low, high) = min_max(42, 17)
say low     # 17
say high    # 42

# Tuple literals and swapping:
let pair = (100, 200)
let x, y = pair
say x       # 100
say y       # 200

# Parallel swap without temporary variable:
x, y = y, x
say x       # 200
say y       # 100
```

---

## 2. Recursion

Functions can freely call themselves. Tail and tree recursion are naturally supported by native CPU stack frames:

```alya
function factorial(n)
    if n <= 1
        return 1
    end
    return n * factorial(n - 1)
end

say factorial(5)    # 120
```

---

## 3. Modules & File Imports

Split large programs into maintainable modular units using `import`:

```alya
# Import by relative path
import "utils/math.alya"

# Import built-in standard library packages
import "std/math"
import "std/str"
import "std/net"
import "std/console"
import "std/glob"
import "std/rand"
```

### Key Module Properties:
1. **Deduplication**: Files imported multiple times across different modules are only parsed and generated once.
2. **Circular Import Protection**: The Alya compiler tracks dependency graphs and detects cycles safely.
3. **Symbol Export**: All top-level functions, structs, and variables declared in an imported file are available to the importer.
4. **Aliasing**: Modules can be aliased to avoid naming conflicts:
   ```alya
   import "modules/logger.alya" as log
   ```

---

## 4. Progressive Examples

### Level 1: Pure & Minimal (Basic Math Utility)
```alya
function square(n)
    return n * n
end

function cube(n)
    return n * square(n)
end

say square(4)    # 16
say cube(3)      # 27
```

---

### Level 2: Practical & Idiomatic (Euclidean GCD & Binary Search)
```alya
# Greatest Common Divisor using Euclid's Algorithm
function gcd(a, b)
    while b != 0
        let temp = b
        b = a % b
        a = temp
    end
    return a
end

# Iterative Binary Search on a sorted array
function binary_search(arr, target)
    let low = 0
    let high = arr.len() - 1

    while low <= high
        let mid = (low + high) / 2
        let val = arr[mid]

        if val == target
            return mid    # Found at index mid
        elif val < target
            low = mid + 1
        else
            high = mid - 1
        end
    end

    return -1   # Target not found
end

let numbers = [10, 23, 35, 47, 59, 72, 88, 99]
let idx = binary_search(numbers, 59)
say "Target 59 found at index: {idx}"    # 4

say "GCD of 48 and 18: " + gcd(48, 18)  # 6
```

---

### Level 3: Advanced & Real-World (Multi-Module E-Commerce Checkout Pipeline)

Here we design a multi-module architecture:

#### Module 1: `modules/tax.alya`
```alya
function compute_tax(subtotal, region_code)
    when region_code
        is 1 then return subtotal * 0.05    # Region 1: 5%
        is 2 then return subtotal * 0.08    # Region 2: 8%
        is 3 then return subtotal * 0.12    # Region 3: 12%
        else return subtotal * 0.07         # Standard default: 7%
    end
end
```

#### Module 2: `modules/discount.alya`
```alya
function apply_coupon(subtotal, code)
    if code == "SAVE20"
        if subtotal >= 100.0
            return subtotal * 0.20   # 20% discount
        end
    elif code == "WELCOME10"
        return 10.0                 # Flat $10 discount
    end
    return 0.0                      # No discount applied
end
```

#### Main Application: `main.alya`
```alya
import "modules/tax.alya"
import "modules/discount.alya"

function process_checkout(customer, raw_subtotal, coupon, region)
    let subtotal = float(raw_subtotal)
    let discount = apply_coupon(subtotal, coupon)
    let discounted_subtotal = subtotal - discount

    let tax = compute_tax(discounted_subtotal, region)
    let final_total = discounted_subtotal + tax

    say "=========================================="
    say "Order Invoice for: {customer}"
    say "Gross Subtotal:     ${subtotal}"
    say "Coupon Applied:     -${discount} ({coupon})"
    say "Net Subtotal:       ${discounted_subtotal}"
    say "Regional Tax:       +${tax}"
    say "Grand Total Due:    ${final_total}"
    say "=========================================="
    return final_total
end

let total = process_checkout("Jane Doe", 150.0, "SAVE20", 2)
```
