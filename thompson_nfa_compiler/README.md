# Thompson NFA Compiler

A Rust-based compiler that converts regular expressions into Thompson Non-deterministic Finite Automata (NFA) with two-character transitions, then generates SystemVerilog for hardware acceleration.

## Features

- **Two-Character Transitions**: Each NFA transition examines both current and lookahead characters
- **Possessive Quantifiers**: Support for `++`, `*+`, `?+` possessive quantifiers (⚠️ currently buggy)
- **SystemVerilog Generation**: Generates synthesizable HDL for FPGA/ASIC implementation
- **Hardware DFS Support**: Designed for hardware-based depth-first search execution
- **Cocotb Integration**: Comprehensive testbench using Python and cocotb

## Architecture

```
Regex Pattern → HIR → Thompson NFA → SystemVerilog → FPGA/ASIC
     ↓              ↓                    ↓              ↓
  "a++b"         [NFA States]        FSM Logic     Hardware
```

### Key Components

1. **Compiler** (`src/compiler.rs`): Converts HIR to Thompson NFA
2. **NFA** (`src/nfa.rs`): NFA data structures with two-character transitions  
3. **SystemVerilog Generator** (`src/verilog_gen.rs`): HDL code generation
4. **Matcher** (`src/matcher.rs`): Software NFA execution for testing

## Usage

### Basic Compilation

```rust
use thompson_nfa_compiler::{Compiler, SystemVerilogGenerator};
use regex_syntax::ParserBuilder;

// Parse regex
let hir = ParserBuilder::new().build().parse("a+b")?;

// Compile to NFA
let nfa = Compiler::new().compile(&hir)?;

// Generate SystemVerilog
let verilog = SystemVerilogGenerator::new()
    .generate_module(&nfa, "my_fsm");
```

### Testing Pipeline

```bash
# Run all tests
make test

# Unit tests only (fast)
make unit-test

# Integration tests with cocotb simulation
make integration-test

# Analyze circuit transitions
make cocotb-test

# Debug possessive quantifiers (currently failing)
make possessive-test
```

## SystemVerilog Interface

Generated modules have this interface:

```systemverilog
module tokenizer_complex(
    input [7:0] current_state,     // Current FSM state
    input [31:0] first_char,       // Current UTF-32 codepoint
    input [31:0] second_char,      // Lookahead UTF-32 codepoint  
    input second_valid,            // Whether lookahead is valid
    
    output [7:0] next_state,       // Primary next state
    output [7:0] second_state,     // Secondary state (for splits)
    output consumed,               // Whether to advance input
    output enabled                 // Whether second_state is valid
);
```

## Testing with Cocotb

The project includes comprehensive cocotb testbenches:

### Pytest + Cocotb Integration

```python
# tests/test_cocotb_integration.py
def test_simple_character_matching(self):
    sv_file = self.create_test_circuit("[abc]", "char_test")
    test_patterns = [
        ("a", True),
        ("b", True), 
        ("x", False),
    ]
    # ... run cocotb simulation
```

### DFS Pattern Matching

The hardware FSM is designed to work with a software DFS controller:

```python
async def dfs_match_pattern(dut, pattern):
    active_states = {47}  # Start state
    
    for char in pattern:
        # Set up inputs
        dut.first_char.value = char_to_utf32(char) 
        
        # Explore all active states
        for state in active_states:
            dut.current_state.value = state
            await Timer(1, units='ns')
            
            # Collect next states based on consumed/enabled/splits
            # ... (see tests/conftest.py for full implementation)
```

## Current Status

### ✅ Working Features

- Basic character classes: `[abc]`, `[sdmt]` 
- Literal strings: `"abc"`, `"hello"`
- Simple alternation: `a|b|c`
- SystemVerilog generation with proper timing
- Cocotb simulation integration
- Comprehensive test coverage

### ⚠️ Known Issues

1. **Possessive Quantifiers Broken**: Patterns like `L++` get stuck in infinite loops (state 10 → 10)
2. **Complex Unicode Classes**: `\\p{L}`, `\\p{N}` mappings need verification  
3. **State Optimization**: Generated circuits have many unreachable states
4. **Timing Analysis**: Critical path analysis needs refinement

### 🔧 Architecture Issues to Fix

Based on test results from `make possessive-test`:

```
--- Single L should match L++: 'L' ---
L@0: state 10 → 10 cons=1  ← INFINITE LOOP!
✗ FAIL: 'L' → False (expected True)
```

The possessive quantifier implementation needs fundamental fixes in the NFA construction logic.

## Hardware Synthesis

Estimated characteristics from Yosys analysis:
- **Logic Gates**: ~5,700 gates for complex tokenizer
- **Critical Path**: ~8-12 gate delays  
- **Max Frequency**: 250-400 MHz (typical FPGA)
- **Resources**: Combinational logic only (no registers)

## Development

### File Structure

```
thompson_nfa_compiler/
├── src/
│   ├── lib.rs              # Library exports
│   ├── nfa.rs             # NFA data structures  
│   ├── compiler.rs        # HIR → NFA compilation
│   ├── matcher.rs         # Software NFA execution
│   ├── verilog_gen.rs     # SystemVerilog generation
│   └── main.rs            # CLI demo
├── tests/
│   ├── conftest.py        # Pytest fixtures & utilities
│   ├── test_regex_to_circuit.py    # End-to-end tests
│   └── test_cocotb_integration.py  # Cocotb simulation tests  
├── test_*.py              # Legacy standalone cocotb tests
├── Makefile              # Main build system
└── pytest.ini           # Pytest configuration
```

### Contributing

1. **Fix Possessive Quantifiers**: The core issue is in `src/compiler.rs` - possessive patterns create infinite loops
2. **Add More Unicode Support**: Extend `CharacterPredicate` for better Unicode class handling
3. **Optimize Generated Circuits**: Reduce state count and improve timing
4. **Add More Test Coverage**: Edge cases, error handling, complex patterns

### Running Tests

```bash
# Quick unit tests
make unit-test

# Full simulation tests (slower)
make integration-test  

# Debug specific issues
make possessive-test    # Shows the possessive bug
make cocotb-test       # Detailed FSM analysis
```

## License

MIT License - see LICENSE file for details.