## Compressed Sparse Row (CSR) data structure

The simulation in a few places uses a CSR data structure. This is a highly efficient way to store and access info about particles which are neighbours. When simulating a system where each particle only interacts with a few neighbors. If you had 10,000 particles, a simple grid representing all possible interactions would be a matrix of 100 million entries ($10,000 \times 10,000$). However, most of those entries would be empty because a particle only "sees" a few neighbors. A matrix where most values are zero or empty is called a sparse matrix. Storing it normally is a waste of memory. CSR is a way to "compress" that matrix so you only store the actual data. One way around this is to use a linked list. Where each particle has a list of its neighbours. This is better than a full matrix, but it still has a lot of overhead because each list is a separate allocation in memory. CSR is even better because it stores all the neighbour IDs in one big array, and then uses another array to tell you where each particle's neighbour list starts and ends. This is much more efficient for both memory and speed.

A CSR represents a sparse matrix (or a list of lists) using two arrays:
- Values (verlet_indices): This is a flat, 1D array that stores all the "actual" neighbour IDs back-to-back. All the "emptiness" is stripped away.
- Offsets (verlet_offsets): This tells you where each particle's neighbour list begins and ends inside the Values array.

Imagine 3 particles with these neighbours:
- Particle 0: neighbors [1, 2]
- Particle 1: neighbor [2]
- Particle 2: neighbors [0, 1]

The "Naive" Way (Wasteful) is to use a Vec<Vec<usize>>. This is easy to read but is memory-fragmented because each inner Vec is a separate little island of memory somewhere else in your computer's RAM.The CSR Way (Efficient) flattens this into two arrays:
- Values: [1, 2, 2, 0, 1]
- Offsets: [0, 2, 3, 5]

To find neighbours for Particle 0 look at Offsets[0] to Offsets[1], which is index 0 to 2. Go to Values[0..2] $\rightarrow$ [1, 2].
To find neighbours for Particle 1: Look at Offsets[1] to Offsets[2], which is index 2 to 3. Go to Values[2..3] $\rightarrow$ [2].
To find neighbours for Particle 2: Look at Offsets[2] to Offsets[3], which is index 3 to 5. Go to Values[3..5] $\rightarrow$ [0, 1].

This is better for the simulation because:
- `Cache Locality`. In your original Vec<Vec<usize>> code, the CPU had to "chase pointers"—jumping from the main Vec to a random location in memory to find the neighbour list. With CSR, all the neighbor IDs are sitting next to each other in one big block. The CPU prefetcher loves this; it can load the next neighbours before your code even asks for them.
- `Memory Overhead`: You have zero overhead from the Vec headers (each Vec object in Rust is 24 bytes). With 10,000 particles, you save a massive amount of memory just by not having 10,000 separate Vec structs.
- `Deterministic`: The memory layout is predictable and static. This makes your code much easier for the compiler to optimise and vectorise.In short, CSR turns a "linked-list-style" mess of heap allocations into a single, clean, high-speed array that your hardware can scan through at blistering speeds.
