#[allow(dead_code)]
pub fn get_expected_output(example_name: &str) -> Option<&'static str> {
    match example_name {
        "arithmetic.alya" => Some(
            "=== Basic Arithmetic ===\n\
20 + 6 = 26\n\
20 - 6 = 14\n\
20 * 6 = 120\n\
20 / 6 = 3\n\
20 % 6 = 2\n\
\n\
=== Expression Precedence ===\n\
(20 + 6) * 2 = 52\n",
        ),
        "arrays.alya" => Some(concat!(
            "Initial array:\n",
            "[10, 20, 30, 40]\n",
            "Array length: 4\n",
            "First element: 10\n",
            "Second element: 20\n",
            "Mutated array:\n",
            "[15, 20, 99, 40]\n",
            "After push operations:\n",
            "[15, 20, 99, 40, 50, 60]\n",
            "Popped element: 60\n",
            "After pop:\n",
            "[15, 20, 99, 40, 50]\n",
            "Traversal with while loop:\n",
            "  Element at 0: 15\n",
            "  Element at 1: 20\n",
            "  Element at 2: 99\n",
            "  Element at 3: 40\n",
            "  Element at 4: 50\n",
            "Sum of elements: 224\n",
            "Caught error safely: index out of bounds\n",
        )),
        "builtins.alya" => Some(
            "Length: 12\n\
Absolute value: 25\n\
Min: 15\n\
Max: 42\n\
Square root of 64: 8\n\
2 to the power of 10: 1024\n\
Trimmed: Alya Programming Language\n\
Uppercase: ALYA PROGRAMMING LANGUAGE\n\
Lowercase: alya programming language\n\
Contains 'Language': 1\n\
Substring: Alya\n\
Split count: 3\n\
Joined: apple - banana - orange\n",
        ),
        "calculator.alya" => Some(
            "=== Simple Calculator ===\n\
Addition:       15 + 7 = 22\n\
Subtraction:    15 - 7 = 8\n\
Multiplication: 15 * 7 = 105\n\
Division:       15 / 7 = 2\n\
Modulo:         15 % 7 = 1\n\
\n\
=== Complex Expressions ===\n\
(15 + 7) * 2 = 44\n\
(15 - 7) / 2 = 4\n\
100 / 4 - 5 = 20\n\
(3 + 5) * (10 - 2) = 64\n",
        ),
        "character_tools.alya" => Some(
            "=== Character Indexing ===\n\
First char: A\n\
Second char: l\n\
\n\
=== ASCII Codes ===\n\
ASCII of 'A': 65\n\
Char from 66: B\n\
\n\
=== Character Classification ===\n\
Is '5' a digit? 1\n\
Is 'a' a digit? 0\n\
Is 'x' alpha? 1\n\
Is '9' alpha? 0\n\
Is 'x' alnum? 1\n\
Is '!' alnum? 0\n\
Is ' ' space? 1\n\
Is 'A' space? 0\n",
        ),
        "cli_args.alya" => Some(
            "=== Command-Line Arguments ===\n\
Argument count: 0\n",
        ),
        "comments.alya" => Some("Sum: 30\n"),
        "compound_operators.alya" => Some(
            "Initial: 10\n\
After += 5: 15\n\
After -= 3: 12\n\
After *= 4: 48\n\
After /= 2: 24\n",
        ),
        "conditionals.alya" => Some(
            "=== Academic Evaluation ===\n\
Score:      85\n\
Attendance: 92%\n\
Result: Grade B - Good job!\n\
Status: Eligible for honors\n\
Award:  Scholarship considered\n",
        ),
        "destructuring.alya" => Some(concat!(
            "=== Array Destructuring ===\n",
            "x: 10, y: 20, z: 30\n\n",
            "=== Rest Pattern ===\n",
            "First: 1\n",
            "Second: 2\n",
            "Tail length: 3\n\n",
            "=== Multiple Returns ===\n",
            "Resolution: 1920x1080\n\n",
            "=== In and Not In Operators ===\n",
            "Has apple: 1\n",
            "Grape not in fruits: 1\n",
        )),
        "enums.alya" => Some(concat!(
            "=== Enums in Alya ===\n",
            "Status.Pending: 1\n",
            "Status.Active: 2\n",
            "Status.Completed: 3\n",
            "Status.Failed: 4\n\n",
            "=== Pattern Matching with Enums ===\n",
            "Active / In Progress\n",
            "Successfully Completed\n\n",
            "=== Direction Enums ===\n",
            "Heading East: 90 degrees\n",
            "Heading South: 180 degrees\n",
        )),
        "fibonacci.alya" => Some(
            "=== Fibonacci Sequence in Alya ===\n\
Computing first 10 Fibonacci numbers with a loop:\n\
0\n\
1\n\
1\n\
2\n\
3\n\
5\n\
8\n\
13\n\
21\n\
34\n\
\n\
Step-by-step recurrence demonstration:\n\
F(5) = F(4) + F(3) = 3 + 2 = 5\n\
F(6) = F(5) + F(4) = 5 + 3 = 8\n\
F(7) = F(6) + F(5) = 8 + 5 = 13\n\
F(8) = F(7) + F(6) = 13 + 8 = 21\n",
        ),
        "file_io.alya" => Some(
            "=== File Existence ===\n\
Exists before writing: 0\n\
\n\
=== Writing File ===\n\
Write successful: 1\n\
Exists after writing: 1\n\
\n\
=== Reading File ===\n\
Hello from Alya File I/O!\n\
Building compilers with self-hosting.\n\
=== Deleting File ===\n\
Delete successful: 1\n\
Exists after deleting: 0\n",
        ),
        "floats.alya" => Some(
            "=== Floating-Point Operations ===\n\
Radius: 2.5\n\
Circumference: 15.708\n\
Area: 19.6349\n\
Average: 25.25\n\
Converted to float: 42.5\n\
Converted back to int: 42\n\
Final score: 30\n",
        ),
        "functions.alya" => Some(
            "=== Functions Demo ===\n\
Hello, Developer! Welcome to Alya.\n\
Area of 8x5 rectangle: 40\n\
Total after discount:  $90\n",
        ),
        "hello.alya" => Some(
            "Hello, World!\n\
Welcome to Alya programming language!\n",
        ),
        "interpolation.alya" => Some(
            "=== Developer Profile ===\n\
Name:        Alice\n\
Role:        Software Engineer\n\
Experience:  5 years\n\
Projects:    12 completed\n\
\n\
Summary: Alice is a Software Engineer with 5 years of experience across 12 projects.\n",
        ),
        "loops.alya" => Some(
            "=== For Loop (Range 1..5) ===\n\
Iteration 1\n\
Iteration 2\n\
Iteration 3\n\
Iteration 4\n\
Iteration 5\n\
\n\
=== While Loop with += ===\n\
Count: 1\n\
Count: 2\n\
Count: 3\n\
Count: 4\n\
\n\
=== While Loop with Break ===\n\
While item: 1\n\
While item: 2\n\
While item: 3\n\
While broke early at w = 4\n\
\n\
=== Repeat Loop with Break ===\n\
Repeat item: 1\n\
Repeat item: 2\n\
Repeat item: 3\n\
Repeat broke at r = 4\n\
\n\
=== Loop with Continue ===\n\
Odd number: 1\n\
Odd number: 3\n\
Odd number: 5\n\
\n\
=== For-Each Array Iteration ===\n\
Fruit: apple\n\
Fruit: banana\n\
Fruit: cherry\n",
        ),
        "main.alya" => Some(
            "Enter name: Hello, TestUser! Welcome to Alya.\n\
Speed: 50 ops/sec\n",
        ),
        "maps.alya" => Some(
            "=== Hash Map Initialization ===\n\
Initial size: 0\n\
\n\
=== Accessing Entries ===\n\
Alice score: 95\n\
Bob score: 88\n\
Charlie score: 70\n\
Size after inserts: 3\n\
\n\
=== Key Membership ===\n\
Has Alice? 1\n\
Has David? 0\n\
\n\
=== Removing Entries ===\n\
Removed Charlie: 1\n\
Has Charlie? 0\n\
Size after removal: 2\n\
\n\
=== Keys and Values ===\n\
Keys count: 2\n\
Values count: 2\n\
Total score: 183\n",
        ),
        "memory_arena_demo.alya" => Some(concat!(
            "=== 1. Testing Raw Memory Allocation & Pointer Access ===\n",
            "Allocated 64 bytes.\n",
            "Peek byte 0 (ascii):\n",
            "72\n",
            "Peek byte 1 (ascii):\n",
            "101\n",
            "String from raw pointer:\n",
            "Hello\n",
            "Peek int at offset 8:\n",
            "987654321\n",
            "Copied string from dest:\n",
            "Hello\n",
            "Copied int from dest offset 8:\n",
            "987654321\n",
            "Byte after zero_mem:\n",
            "0\n",
            "Raw buffers successfully freed!\n\n",
            "=== 2. Testing High-Performance Arena Allocator ===\n",
            "Block 1 string:\n",
            "Alya\n",
            "Block 2 int value:\n",
            "2026\n",
            "Total bytes allocated in arena:\n",
            "80\n",
            "Total bytes allocated after reset:\n",
            "0\n",
            "Re-allocated block string:\n",
            "Z\n",
            "Arena successfully destroyed!\n\n",
            "Advanced Memory Management demonstration finished successfully!\n",
        )),
        "mini_compiler.alya" => Some(concat!(
            "==================================================\n",
            "      Alya Mini-Compiler (Written in Alya)        \n",
            "==================================================\n",
            "No file provided. Compiling embedded demo program:\n",
            "1. Tokenizing source code...\n",
            "   Generated 53 tokens.\n",
            "2. Parsing and generating x64 Assembly...\n",
            "3. Writing native assembly to mini_output.s...\n\n",
            "[SUCCESS] Compilation complete!\n",
            "To produce a native standalone binary, run:\n",
            "   gcc mini_output.s -o mini_program.exe\n",
            "   ./mini_program.exe\n",
            "==================================================\n",
        )),
        "modern_features.alya" => Some(
            "Calculated score: 46\n\
Access granted!\n\
Length of greeting: 13\n\
Absolute value of -42: 42\n\
Minimum of 10 and 20: 10\n\
Maximum of 10 and 20: 20\n",
        ),
        "modern_syntax.alya" => Some(concat!(
            "=== 1. Ternary & Inline If ===\n",
            "Grade: Passed\n",
            "Label: non-positive\n",
            "Max: 25\n\n",
            "=== 2. Default Parameters ===\n",
            "Hello, World!\n",
            "Welcome, Alice!\n",
            "Good day, Bob?\n",
            "Square of 5: 25\n",
            "Cube of 3: 27\n\n",
            "=== 3. Multiline & Raw Strings ===\n",
            "*-------------------*\n",
            "| Welcome to Alya!  |\n",
            "*-------------------*\n\n",
            "Raw path: C:\\path\\to\\config.json\n\n",
            "=== 4. Null Coalescing (??) ===\n",
            "Port: 8080\n",
            "User: Anonymous\n",
            "Host: localhost\n",
        )),
        "modules.alya" => Some(
            "=== Modules & Imports ===\n\
Sum: 16\n\
Product: 48\n\
Square of 12: 144\n\
Inline call: 2\n",
        ),
        "null_and_bitwise.alya" => Some(concat!(
            "Initial value: null\n",
            "Value is null\n",
            "Updated value: 42\n",
            "Value is not null\n",
            "Bitwise AND: 8\n",
            "Bitwise OR:  14\n",
            "Bitwise XOR: 6\n",
            "Shift left:  16\n",
            "Shift right: 8\n",
            "After &= 3: 3\n",
            "After |= 8: 11\n",
            "After ^= 2: 9\n",
            "After <<= 2: 36\n",
            "After >>= 3: 4\n",
        )),
        "pattern_matching.alya" => Some(
            "=== HTTP Status Code Resolver ===\n\
Success response received\n\
Status 201: OK / Created\n\
\n\
=== Priority Level Resolver ===\n\
Priority 2: Medium\n",
        ),
        "quickstart_arithmetic.alya" => Some("15\n5\n50\n2\n"),
        "struct_methods.alya" => Some(concat!(
            "=== Vector2D Methods ===\n",
            "Vector2D(3, 4)\n",
            "Length squared: 25\n",
            "After adding (10, 20):\n",
            "Vector2D(13, 24)\n\n",
            "=== BankAccount Methods ===\n",
            "Initial balance: 100\n",
            "After deposit 50: 150\n",
            "Withdraw 30 ok: 1\n",
            "Final balance: 120\n",
        )),
        "structs.alya" => Some(concat!(
            "=== Structs and Custom Types ===\n",
            "Point 1:\n",
            "Point { x: 10, y: 20 }\n",
            "Point 1 coordinates: (10, 20)\n",
            "Mutated Point 1:\n",
            "Point { x: 100, y: 25 }\n",
            "Point 2: (5, 12)\n",
            "Distance squared of Point 2: 169\n",
            "Shifted Point: (15, 32)\n",
            "Alice is a Systems Architect with 8 years of experience.\n",
        )),
        "stdlib_demo.alya" => Some(concat!(
            "=== 1. Testing std/math ===\n",
            "PI is:\n",
            "3.14159\n",
            "180 degrees in radians:\n",
            "3.14159\n",
            "Back to degrees:\n",
            "180\n",
            "Clamp 15 between 1 and 10:\n",
            "10\n",
            "Hypotenuse of 3 and 4:\n",
            "5\n",
            "Is 42 even?\n",
            "1\n\n",
            "=== 2. Testing std/time ===\n",
            "Current epoch time > 0:\n",
            "1\n",
            "Sleeping for 10ms...\n",
            "Sleep finished!\n\n",
            "=== 3. Testing std/os ===\n",
            "PATH environment variable is present: 1\n\n",
            "=== 4. Testing std/json ===\n",
            "JSON number:\n",
            "123\n",
            "JSON string:\n",
            "\"hello\"\n",
            "JSON boolean:\n",
            "true\n",
            "JSON array:\n",
            "[1, 2, 3]\n\n",
            "All Standard Library modules executed successfully!\n",
        )),
        "stdlib_power_demo.alya" => Some(concat!(
            "=== Test Suite: Alya Extended Standard Library ===\n",
            "  [PASS] starts_with prefix match\n",
            "  [PASS] starts_with non-match\n",
            "  [PASS] ends_with suffix match\n",
            "  [PASS] replace substring\n",
            "  [PASS] str_repeat string\n",
            "  [PASS] pad_left zeros\n",
            "  [PASS] pad_right dots\n",
            "  [PASS] capitalize string\n",
            "  [PASS] count_matches single char\n",
            "  [PASS] is_empty on empty string\n",
            "  [PASS] is_empty on non-empty string\n",
            "  [PASS] lines length\n",
            "  [PASS] lines first item\n",
            "  [PASS] lines third item\n",
            "  [PASS] path_join normal\n",
            "  [PASS] file_name\n",
            "  [PASS] file_ext\n",
            "  [PASS] file_stem\n",
            "  [PASS] parent_dir\n",
            "  [PASS] is_absolute unix root\n",
            "  [PASS] floor positive\n",
            "  [PASS] ceil positive\n",
            "  [PASS] trunc positive\n",
            "  [PASS] sin(0) is 0\n",
            "  [PASS] cos(0) is 1\n",
            "  [PASS] rand_range bounds\n",
            "  [PASS] sum of numbers\n",
            "  [PASS] mean of numbers\n",
            "  [PASS] median of numbers\n",
            "  [PASS] djb2 deterministic\n",
            "  [PASS] djb2 distinct\n",
            "  [PASS] fnv1a deterministic\n",
            "  [PASS] hex_encode 'Hi'\n",
            "  [PASS] hex_decode '4869'\n",
            "  [PASS] base64_encode 'Alya'\n",
            "  [PASS] base64_decode 'QWx5YQ=='\n",
            "  [PASS] stack size is 2\n",
            "  [PASS] stack peek top\n",
            "  [PASS] stack pop top\n",
            "  [PASS] stack pop last\n",
            "  [PASS] stack is empty\n",
            "  [PASS] queue size is 2\n",
            "  [PASS] queue peek FIFO\n",
            "  [PASS] queue pop FIFO\n",
            "  [PASS] queue pop next\n",
            "  [PASS] queue is empty\n",
            "  [PASS] set has alpha\n",
            "  [PASS] set does not have gamma\n",
            "  [PASS] set size deduplicated\n",
            "  [PASS] set remove alpha\n",
            "  [PASS] json_object serializes map\n",
            "  [PASS] fs_exists after write\n",
            "  [PASS] fs_read matches written content\n",
            "  [PASS] fs_size is 11 bytes\n",
            "  [PASS] copy_file copied content\n",
            "  [PASS] fs_remove cleaned up original\n",
            "  [PASS] fs_remove cleaned up copy\n",
            "----------------------------------------\n",
            "Status: ALL TESTS PASSED!\n",
            "----------------------------------------\n",
            "Test suite finished with code: 0\n",
        )),
        "try_catch.alya" => Some(
            "=== Try-Catch Demo ===\n\
\n\
1. Basic try-catch:\n\
Attempting division...\n\
Caught error: Division by zero was safely handled!\n\
\n\
2. Try-catch with error message:\n\
Calculating modulo...\n\
Caught exception message: division by zero\n\
\n\
3. Successful try block:\n\
Safe division result: 25\n\
\n\
4. Custom throw:\n\
Caught custom error: custom business error\n\
\n\
5. Try-catch-finally:\n\
In try block...\n\
Handled: io error\n\
Finally block executed!\n\
\n\
Program completed successfully without crashing.\n",
        ),
        "user_input.alya" => Some(
            "What is your name? Hello, TestUser! Welcome to Alya.\n\
What is your favorite programming language? Awesome, Alya is great!\n",
        ),
        "variables.alya" => Some(
            "=== Language Information ===\n\
Language:    Alya\n\
Version:     1\n\
Year:        2026\n\
Open Source: 1\n\
Next version will be: 2\n\
Project age: 2 years\n",
        ),
        _ => None,
    }
}
