# Project Overview: Molecular Dynamics Simulation

## 1. Domain & Simulation Goals (What we are achieving)
- Trying to build an API which leaves the user in their scripts just writing the bits that differ. They define interactions and choose what to output. 
- Trying to make sure we can reconstruct the entire history of simulation through logging all .json files at start and end of simulation or when anything changes. Log particle coords at regular intervals.
- Within a simulation prefer all parameters to be changed from configuration files (.json) rather than be hardcoded.
- Graphics and simulation are kept separate domains.

## 2. System Architecture & Component Roles (How it connects)
- **Rust (`src/`)**: High-performance core engine. Responsible for neighbor lists (cell lists / Verlet lists), force/torque calculations, and trajectory integration.
- **Python (`scripts/` or `setup/`)**: Initial state generation (e.g., random packings, lattice placement), parameter sweeps, and post-processing/analysis (RDF, order parameters).
- **Bash**: Job orchestration, compilation scripts.

## 3. Engineering & Performance Guardrails (Crucial Constraints)
- **Data Layout**: Prefer Struct-of-Arrays (SoA) or contiguous flat slices for particle positions/orientations to stay cache- and SIMD-friendly or Compressed Sparse Arrays or DashMaps
- **Orientational Dynamics**: Use quaternions, rotation matrices

## 4. Collaboration & Development Workflow

### A. Clarification & Quizzing First
- For any non-trivial change, **do not jump straight into writing code**.
- Quiz me with targeted questions about edge cases, physics assumptions, performance tradeoffs, and API preferences before drafting solutions.
- Use multiple-choice questions or concrete tradeoffs whenever applicable.

### B. Plan First, Code Second
- Before writing code, present a **high-level plan or architectural overview** detailing:
  1. The affected components and data flow.
  2. The proposed steps broken down into small, self-contained increments.
- Wait for my approval or feedback on the plan before implementing.

### C. Incremental Implementation & Verification
- Implement changes in small, isolated steps (one function, trait, or file at a time).
- After each incremental step, discuss with me whether any new tests are appropriate to confirm everythings working.
- Do not make speculative or sprawling edits across multiple modules in a single step.
