# Project style guide

## Project Organization & Testing Conventions

Every major subdirectory containing a `mod.rs` file should include a corresponding `test.rs` file dedicated to unit-testing the modules within that folder. 

- md_viz --> Anything that concerns graphics
- md_sim --> Anything that concerns the simulation

Within md_sim:
- force --> Calculating forces (Force trait)
- motion --> Integrating motion (Motion trait)
- particle --> Objects in the simulation
- utils --> file input / output, etc (Interactivity trait)


## Structs and Enums

Every public struct and enum should be preceded by a header banner containing its name, followed by a concise description of its purpose and key fields.

Template
```Rust
/// ==============================================================================
/// StructName
/// ==============================================================================
/// Brief description of what the structure represents and its primary role.
///
/// Fields:
/// * `field_name` - Description of the field and its units or constraints.
#[derive(Debug, Clone)]
pub struct StructName {
    pub field_name: Type,
}
```

## Functions and Methods

Standalone functions and impl methods use a banner matching the item name, followed by a detailed description, arguments, and return values.

Template
```Rust
/// ==============================================================================
/// function_name
/// ==============================================================================
/// Brief description of what the function computes or performs.
///
/// Detailed explanation of the underlying algorithm, physical model, or 
/// numerical approximations used (if applicable).
///
/// # Arguments
///
/// * `arg_name` - Description of the argument, including expected ranges or units.
///
/// # Returns
///
/// Description of the return value and its physical meaning.
pub fn function_name(arg_name: Type) -> ReturnType {
    // Implementation
}
```

## Implementation Block Section Dividers

When organizing large impl blocks (e.g., separating public APIs from internal helpers), use clear comment banners to divide the code logically.

Template
```Rust
impl MyStruct {
    // ======================================================================
    // Public API
    // ======================================================================

    /// ==================================================================
    /// method_name
    /// ==================================================================
    pub fn method_name(&self) {
        // ...
    }

    // ======================================================================
    // Internal Helper Methods
    // ======================================================================

    fn helper_method(&self) {
        // ...
    }
}
```

## Test Documentation

This section outlines the standardised documentation format for all tests across the codebase. While source code uses item-name headers, test functions use a dedicated What / How / Why banner to clearly explain the test scenario and its physical or logical verification goals at a glance.

## Test Documentation Template

Every test function must be preceded by a structured Markdown banner block using equal signs (=) for borders, followed by three explicit sections:

```Rust
/// ====================================================================================================================
/// **What:** [Concise statement of the specific behavior, feature, or edge case being verified]  
/// **How:** [Description of the test setup, input parameters, and execution scenarios—including positive and negative checks]  
/// **Why:** [The underlying physical, numerical, or architectural rationale for why this test is critical]
/// ====================================================================================================================
#[test]
fn test_feature_behavior() {
    // Test implementation
}
```

### Section Breakdown

**What:**

State precisely what component or mechanism is under test (e.g., particle-to-particle collision detection, contact geometry resolution, and non-contact filtering).

**How:**

Explain the test setup and scenarios covered. Mention how different states are tested (e.g., tests an overlapping particle pair for positive contact properties and a separated pair to verify None assertions).

**Why:**

Explain the significance of the test (e.g., ensures that particle interactions correctly compute overlap depth and relative kinematics while preventing false positives on separated bodies).

## Summary of Conventions
Separator Lines: Use equal signs (=) for major structural/function headers (/// ==============================================================================) to ensure high visual distinctiveness in the editor.

Item Name Headers: Place the exact identifier name on its own line between the separator borders.

Mathematical Notation: Use standard Markdown or LaTeX formatting where appropriate for equations (e.g., $F_n$ or $\mu F_n$).

Standard Sections: Always include # Arguments and # Returns sections for functions/methods that take parameters and produce outputs.

