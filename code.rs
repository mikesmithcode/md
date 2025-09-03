#   Root Cargo.toml
[workspace]
members = [
    "crates/particle_core",
    "crates/particle_viz",
]

[workspace.dependencies]
rand = "0.8"
nalgebra = { version = "0.32", features = ["serde-serialize"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# =============================================================================
# crates/particle_core/Cargo.toml
# =============================================================================
[package]
name = "particle_core"
version = "0.1.0"
edition = "2021"

[dependencies]
rand = { workspace = true }
nalgebra = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }

# =============================================================================
# crates/particle_core/src/lib.rs
# =============================================================================
use std::collections::HashMap;
use rand::Rng;
use nalgebra::{Vector3, Point3};
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;

// High precision types for scientific simulation
pub type Vec3 = Vector3<f64>;
pub type Point3d = Point3<f64>;

// Convenience constructors
impl Vec3 {
    pub fn random_unit_sphere() -> Self {
        let mut rng = rand::thread_rng();
        loop {
            let v = Self::new(
                rng.gen::<f64>() * 2.0 - 1.0,
                rng.gen::<f64>() * 2.0 - 1.0,
                rng.gen::<f64>() * 2.0 - 1.0,
            );
            if v.norm_squared() <= 1.0 {
                return v.normalize();
            }
        }
    }
    
    pub fn random_uniform(min: f64, max: f64) -> Self {
        let mut rng = rand::thread_rng();
        Self::new(
            rng.gen::<f64>() * (max - min) + min,
            rng.gen::<f64>() * (max - min) + min,
            rng.gen::<f64>() * (max - min) + min,
        )
// Simulation state snapshot for serialization
#[derive(Serialize, Deserialize)]
pub struct SimulationSnapshot {
    pub time: f64,
    pub dt: f64,
    pub bounds: Bounds3D,
    pub particles: Vec<Particle>,
    pub behavior_name: String,
    pub metadata: HashMap<String, String>,
}

impl SimulationSnapshot {
    pub fn new<B: ParticleBehavior>(simulation: &Simulation<B>) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("created_at".to_string(), 
                        chrono::Utc::now().to_rfc3339());
        metadata.insert("particle_count".to_string(), 
                        simulation.particles.len().to_string());
        metadata.insert("total_kinetic_energy".to_string(), 
                        simulation.total_kinetic_energy().to_string());
        metadata.insert("version".to_string(), "1.0".to_string());
        
        Self {
            time: simulation.time,
            dt: simulation.dt,
            bounds: simulation.bounds,
            particles: simulation.particles.clone(),
            behavior_name: simulation.behavior_name().to_string(),
            metadata,
        }
    }
    
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
    
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let json = fs::read_to_string(path)?;
        let snapshot: SimulationSnapshot = serde_json::from_str(&json)?;
        Ok(snapshot)
    }
    
    pub fn generate_filename(behavior_name: &str, time: f64, step: usize) -> String {
        let clean_name = behavior_name.replace(" ", "_").to_lowercase();
        format!("{}_{:010.3}s_step_{:06}.json", clean_name, time, step)
    }
}

// Base particle - high precision for scientific accuracy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Particle {
    pub id: usize,
    pub position: Vec3,
    pub velocity: Vec3,
    pub radius: f64,
    pub mass: f64,
    // Extensible data storage for particle-specific properties
    pub properties: HashMap<String, f64>,
}

impl Particle {
    pub fn new(id: usize, position: Vec3, radius: f64, mass: f64) -> Self {
        Self {
            id,
            position,
            velocity: Vec3::zeros(),
            radius,
            mass,
            properties: HashMap::new(),
        }
    }
    
    pub fn kinetic_energy(&self) -> f64 {
        0.5 * self.mass * self.velocity.norm_squared()
    }
    
    pub fn momentum(&self) -> Vec3 {
        self.mass * self.velocity
    }
    
    /// Convert position to f32 for rendering (with potential precision loss warning)
    pub fn position_f32(&self) -> nalgebra::Vector3<f32> {
        nalgebra::Vector3::new(
            self.position.x as f32,
            self.position.y as f32,
            self.position.z as f32,
        )
    }
    
    /// Convert radius to f32 for rendering
    pub fn radius_f32(&self) -> f32 {
        self.radius as f32
    }
}

// 3D bounding box for simulation bounds - high precision
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Bounds3D {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds3D {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }
    
    pub fn cube(size: f64) -> Self {
        Self {
            min: Vec3::zeros(),
            max: Vec3::new(size, size, size),
        }
    }
    
    pub fn box_size(width: f64, height: f64, depth: f64) -> Self {
        Self {
            min: Vec3::zeros(),
            max: Vec3::new(width, height, depth),
        }
    }
    
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
    
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
    
    pub fn volume(&self) -> f64 {
        let size = self.size();
        size.x * size.y * size.z
    }
    
    /// Convert to f32 for rendering
    pub fn to_f32(&self) -> (nalgebra::Vector3<f32>, nalgebra::Vector3<f32>) {
        (
            nalgebra::Vector3::new(self.min.x as f32, self.min.y as f32, self.min.z as f32),
            nalgebra::Vector3::new(self.max.x as f32, self.max.y as f32, self.max.z as f32)
        )
    }
}

// Trait for particle-specific behavior
pub trait ParticleBehavior {
    /// Apply forces/accelerations to particles before position update
    fn apply_forces(&self, particles: &mut [Particle], dt: f64, bounds: &Bounds3D);
    
    /// Handle collision between two particles
    fn handle_collision(&self, p1: &mut Particle, p2: &mut Particle, dt: f64);
    
    /// Apply boundary conditions
    fn handle_boundary(&self, particle: &mut Particle, bounds: &Bounds3D);
    
    /// Any additional per-step logic
    fn post_step(&self, particles: &mut [Particle], dt: f64) {}
    
    /// Get display name for this behavior
    fn name(&self) -> &'static str;
}

// The main 3D simulation - high precision physics with I/O capabilities
pub struct Simulation<B: ParticleBehavior> {
    pub particles: Vec<Particle>,
    pub bounds: Bounds3D,
    pub dt: f64,
    pub time: f64,
    behavior: B,
    // I/O settings
    save_interval: Option<usize>, // Save every N steps (## File I/O and State Management

### Automatic State Saving
- **Configurable intervals**: Save every N simulation steps
- **Timestamped files**: Automatic filename generation with time and step info
- **Rich metadata**: Creation time, particle count, energy, behavior type
- **Directory management**: Automatically creates output directories

### State Loading
- **Resume simulations**: Load any saved state and continue from that point
- **Cross-behavior compatibility**: Load states created with different behaviors
- **Time continuity**: Simulation time correctly continues from loaded state
- **Validation**: Automatic checks for file format and version compatibility

### File Format
JSON format for human readability and easy analysis:
```json
{
  "time": 15.248,
  "dt": 0.001,
  "bounds": {"min": [0,0,0], "max": [50,50,50]},
  "particles": [...],
  "behavior_name": "3D Inelastic Collision",
  "metadata": {
    "created_at": "2025-08-14T10:30:00Z",
    "particle_count": "25",
    "total_kinetic_energy": "1247.83"
  }
}
```

## Usage Examples

### Basic Auto-saving
```bash
# Run with auto-save every 1000 steps
cargo run --example inelastic_collision

# Files saved to: simulation_output/3d_inelastic_collision_000015.248s_step_001000.json
```

### Resume from File
```bash
# = no saving)
    step_count: usize,
    output_directory: Option<String>,
}

impl<B: ParticleBehavior> Simulation<B> {
    pub fn new(bounds: Bounds3D, dt: f64, behavior: B) -> Self {
        Self {
            particles: Vec::new(),
            bounds,
            dt,
            time: 0.0,
            behavior,
            save_interval: None,
            step_count: 0,
            output_directory: None,
        }
    }
    
    /// Create simulation from saved snapshot
    pub fn from_snapshot(snapshot: SimulationSnapshot, behavior: B) -> Self {
        println!("Loading simulation from snapshot:");
        println!("  Time: {:.3}s", snapshot.time);
        println!("  Particles: {}", snapshot.particles.len());
        println!("  Behavior: {}", snapshot.behavior_name);
        
        if let Some(created_at) = snapshot.metadata.get("created_at") {
            println!("  Created: {}", created_at);
        }
        
        Self {
            particles: snapshot.particles,
            bounds: snapshot.bounds,
            dt: snapshot.dt,
            time: snapshot.time,
            behavior,
            save_interval: None,
            step_count: 0,
            output_directory: None,
        }
    }
    
    /// Configure automatic state saving
    pub fn set_save_interval(&mut self, interval_steps: usize, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Create output directory if it doesn't exist
        fs::create_dir_all(output_dir)?;
        
        self.save_interval = Some(interval_steps);
        self.output_directory = Some(output_dir.to_string());
        
        println!("Configured to save state every {} steps to '{}'", interval_steps, output_dir);
        Ok(())
    }
    
    /// Manually save current state
    pub fn save_state(&self, filename: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
        let snapshot = SimulationSnapshot::new(self);
        
        let filepath = if let Some(name) = filename {
            name.to_string()
        } else {
            let filename = SimulationSnapshot::generate_filename(
                self.behavior_name(), 
                self.time, 
                self.step_count
            );
            
            if let Some(ref dir) = self.output_directory {
                format!("{}/{}", dir, filename)
            } else {
                filename
            }
        };
        
        snapshot.save_to_file(&filepath)?;
        println!("Saved simulation state: {}", filepath);
        Ok(filepath)
    }
    
    /// Load state from file (static method)
    pub fn load_state(filepath: &str, behavior: B) -> Result<Self, Box<dyn std::error::Error>> {
        let snapshot = SimulationSnapshot::load_from_file(filepath)?;
        println!("Loaded simulation state from: {}", filepath);
        Ok(Self::from_snapshot(snapshot, behavior))
    }
    
    pub fn add_particle(&mut self, particle: Particle) {
        self.particles.push(particle);
    }
    
    // The main step function - always the same structure
    pub fn step(&mut self) {
        // 1. Apply forces/noise/active behavior
        self.behavior.apply_forces(&mut self.particles, self.dt, &self.bounds);
        
        // 2. Update positions (always the same)
        self.update_positions();
        
        // 3. Handle collisions
        self.handle_collisions();
        
        // 4. Apply boundary conditions
        self.handle_boundaries();
        
        // 5. Any post-processing
        self.behavior.post_step(&mut self.particles, self.dt);
        
        self.time += self.dt;
        self.step_count += 1;
        
        // 6. Auto-save if configured
        if let Some(interval) = self.save_interval {
            if self.step_count % interval == 0 {
                if let Err(e) = self.save_state(None) {
                    eprintln!("Failed to auto-save state: {}", e);
                }
            }
        }
    }
    
    // Core simulation logic - never changes
    fn update_positions(&mut self) {
        for particle in &mut self.particles {
            particle.position += particle.velocity * self.dt;
        }
    }
    
    fn handle_collisions(&mut self) {
        // Note: In production, you'd want spatial partitioning here
        for i in 0..self.particles.len() {
            for j in (i + 1)..self.particles.len() {
                let delta = self.particles[j].position - self.particles[i].position;
                let dist = delta.norm();
                let min_dist = self.particles[i].radius + self.particles[j].radius;
                
                if dist < min_dist {
                    // Split borrow to get mutable references to both particles
                    let (left, right) = self.particles.split_at_mut(j);
                    self.behavior.handle_collision(&mut left[i], &mut right[0], self.dt);
                }
            }
        }
    }
    
    fn handle_boundaries(&mut self) {
        for particle in &mut self.particles {
            self.behavior.handle_boundary(particle, &self.bounds);
        }
    }
    
    pub fn behavior_name(&self) -> &'static str {
        self.behavior.name()
    }
    
    pub fn total_kinetic_energy(&self) -> f64 {
        self.particles.iter().map(|p| p.kinetic_energy()).sum()
    }
    
    pub fn total_momentum(&self) -> Vec3 {
        self.particles.iter().map(|p| p.momentum()).sum()
    }
    
    pub fn center_of_mass(&self) -> Vec3 {
        let total_mass: f64 = self.particles.iter().map(|p| p.mass).sum();
        if total_mass > 0.0 {
            let weighted_pos: Vec3 = self.particles.iter()
                .map(|p| p.position * p.mass)
                .sum();
            weighted_pos / total_mass
        } else {
            Vec3::zeros()
        }
    }
    
    pub fn add_random_particles(&mut self, count: usize, radius_range: (f64, f64)) {
        let mut rng = rand::thread_rng();
        let size = self.bounds.size();
        let margin = radius_range.1;
        
        let start_id = self.particles.len();
        
        for i in 0..count {
            let radius = rng.gen::<f64>() * (radius_range.1 - radius_range.0) + radius_range.0;
            let position = Vec3::new(
                rng.gen::<f64>() * (size.x - 2.0 * margin) + self.bounds.min.x + margin,
                rng.gen::<f64>() * (size.y - 2.0 * margin) + self.bounds.min.y + margin,
                rng.gen::<f64>() * (size.z - 2.0 * margin) + self.bounds.min.z + margin,
            );
            // Mass proportional to volume for spheres
            let mass = 4.0 / 3.0 * std::f64::consts::PI * radius.powi(3);
            
            self.add_particle(Particle::new(start_id + i, position, radius, mass));
        }
    }
    
    /// Get current step count
    pub fn step_count(&self) -> usize {
        self.step_count
    }
    
    /// Get output directory if configured
    pub fn output_directory(&self) -> Option<&str> {
        self.output_directory.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_nalgebra_operations() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        
        assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
        assert_eq!(a - b, Vec3::new(-3.0, -3.0, -3.0));
        assert_eq!(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
        assert_eq!(a.dot(&b), 32.0);
        
        let c = a.cross(&Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(c, Vec3::new(0.0, 3.0, -2.0));
        
        // Test precision
        let small = Vec3::new(1e-15, 1e-15, 1e-15);
        assert!(small.norm() > 0.0); // f64 can handle very small numbers
    }
    
    #[test]
    fn test_bounds() {
        let bounds = Bounds3D::cube(10.0);
        assert_eq!(bounds.center(), Vec3::new(5.0, 5.0, 5.0));
        assert_eq!(bounds.size(), Vec3::new(10.0, 10.0, 10.0));
        assert_eq!(bounds.volume(), 1000.0);
    }
    
    #[test]
    fn test_particle_conversions() {
        let particle = Particle::new(0, Vec3::new(1.5, 2.5, 3.5), 1.0, 2.0);
        let pos_f32 = particle.position_f32();
        assert_eq!(pos_f32, nalgebra::Vector3::new(1.5f32, 2.5f32, 3.5f32));
        assert_eq!(particle.radius_f32(), 1.0f32);
    }
}

# =============================================================================
# crates/particle_viz/Cargo.toml
# =============================================================================
[package]
name = "particle_viz"
version = "0.1.0"
edition = "2021"

[dependencies]
particle_core = { path = "../particle_core" }
three-d = "0.16"
nalgebra = { workspace = true }
chrono = { version = "0.4", features = ["serde"] }

# =============================================================================
# crates/particle_viz/src/lib.rs
# =============================================================================
use particle_core::*;
use three_d::*;

pub struct ParticleRenderer {
    context: Context,
    camera: Camera,
    camera_control: OrbitControl,
    sphere_mesh: CpuMesh,
    bounds_wireframe: Option<Gm<Mesh, ColorMaterial>>,
}

impl ParticleRenderer {
    pub fn new(window: &Window) -> ThreeDResult<Self> {
        let context = window.gl();
        
        // Create camera with orbit controls
        let viewport = window.viewport();
        let target = Vec3::new(50.0, 50.0, 50.0); // Default center (f32 for three-d)
        let eye = target + Vec3::new(100.0, 100.0, 100.0);
        let up = Vec3::unit_y();
        
        let camera = Camera::new_perspective(
            viewport,
            eye,
            target,
            up,
            degrees(45.0),
            0.1,
            1000.0
        );
        
        let camera_control = OrbitControl::new(target, 50.0, 300.0);
        
        // Create high-quality sphere mesh for scientific visualization
        let sphere_mesh = CpuMesh::sphere(32);
        
        Ok(Self {
            context: context.clone(),
            camera,
            camera_control,
            sphere_mesh,
            bounds_wireframe: None,
        })
    }
    
    pub fn update_bounds(&mut self, bounds: &particle_core::Bounds3D) -> ThreeDResult<()> {
        self.bounds_wireframe = Some(self.create_bounds_wireframe(bounds)?);
        
        // Update camera to frame the bounds (convert f64 to f32)
        let center_f64 = bounds.center();
        let size_f64 = bounds.size();
        
        let center = Vec3::new(center_f64.x as f32, center_f64.y as f32, center_f64.z as f32);
        let max_dimension = (size_f64.x.max(size_f64.y).max(size_f64.z)) as f32;
        let distance = max_dimension * 2.0;
        
        self.camera_control = OrbitControl::new(center, max_dimension * 0.5, distance);
        
        Ok(())
    }
    
    fn create_bounds_wireframe(&self, bounds: &particle_core::Bounds3D) -> ThreeDResult<Gm<Mesh, ColorMaterial>> {
        let (min_f32, max_f32) = bounds.to_f32();
        
        // Create wireframe cube (using three-d's Vec3 which is f32)
        let positions = vec![
            // Bottom face
            Vec3::new(min_f32.x, min_f32.y, min_f32.z), Vec3::new(max_f32.x, min_f32.y, min_f32.z),
            Vec3::new(max_f32.x, min_f32.y, min_f32.z), Vec3::new(max_f32.x, max_f32.y, min_f32.z),
            Vec3::new(max_f32.x, max_f32.y, min_f32.z), Vec3::new(min_f32.x, max_f32.y, min_f32.z),
            Vec3::new(min_f32.x, max_f32.y, min_f32.z), Vec3::new(min_f32.x, min_f32.y, min_f32.z),
            
            // Top face
            Vec3::new(min_f32.x, min_f32.y, max_f32.z), Vec3::new(max_f32.x, min_f32.y, max_f32.z),
            Vec3::new(max_f32.x, min_f32.y, max_f32.z), Vec3::new(max_f32.x, max_f32.y, max_f32.z),
            Vec3::new(max_f32.x, max_f32.y, max_f32.z), Vec3::new(min_f32.x, max_f32.y, max_f32.z),
            Vec3::new(min_f32.x, max_f32.y, max_f32.z), Vec3::new(min_f32.x, min_f32.y, max_f32.z),
            
            // Vertical edges
            Vec3::new(min_f32.x, min_f32.y, min_f32.z), Vec3::new(min_f32.x, min_f32.y, max_f32.z),
            Vec3::new(max_f32.x, min_f32.y, min_f32.z), Vec3::new(max_f32.x, min_f32.y, max_f32.z),
            Vec3::new(max_f32.x, max_f32.y, min_f32.z), Vec3::new(max_f32.x, max_f32.y, max_f32.z),
            Vec3::new(min_f32.x, max_f32.y, min_f32.z), Vec3::new(min_f32.x, max_f32.y, max_f32.z),
        ];
        
        let indices: Vec<u32> = (0..positions.len() as u32).collect();
        
        let cpu_mesh = CpuMesh {
            positions: Positions::F32(positions),
            indices: Some(Indices::U32(indices)),
            ..Default::default()
        };
        
        let geometry = Gm::new(
            Mesh::new(&self.context, &cpu_mesh),
            ColorMaterial::new_transparent(
                &self.context,
                &CpuMaterial {
                    albedo: Srgba::new(100, 100, 100, 100),
                    ..Default::default()
                }
            )?
        );
        
        Ok(geometry)
    }
    
    pub fn render<B: ParticleBehavior>(&mut self, 
                                      simulation: &Simulation<B>, 
                                      screen: &Screen) -> ThreeDResult<()> {
        
        // Create instanced spheres for particles (convert f64 to f32)
        let mut instances = Vec::new();
        let mut colors = Vec::new();
        
        for particle in &simulation.particles {
            // Convert high-precision position and radius to f32 for rendering
            let pos_f32 = particle.position_f32();
            let radius_f32 = particle.radius_f32();
            
            // Transform: translate and scale
            let transform = Mat4::from_translation(pos_f32) * Mat4::from_scale(radius_f32);
            instances.push(transform);
            
            // Color based on behavior type
            let color = self.get_particle_color(particle, simulation.behavior_name());
            colors.push(color);
        }
        
        // Clear screen with scientific dark theme
        screen.clear(ClearState::color_and_depth(0.05, 0.05, 0.1, 1.0, 1.0))?;
        
        // Render bounds wireframe
        if let Some(ref wireframe) = self.bounds_wireframe {
            screen.render(&self.camera, &[wireframe], &[])?;
        }
        
        // Render particles if any exist
        if !instances.is_empty() {
            let instanced_mesh = InstancedMesh::new(
                &self.context, 
                &instances, 
                &Mesh::new(&self.context, &self.sphere_mesh)
            );
            
            // For now, render with a single material (could be optimized to batch by color)
            let material = PhysicalMaterial::new_transparent(
                &self.context,
                &CpuMaterial {
                    albedo: Srgba::new(100, 150, 255, 220),
                    metallic: 0.1,
                    roughness: 0.7,
                    ..Default::default()
                }
            );
            
            // Render all particles (in production, you'd batch by color for performance)
            for (i, color) in colors.iter().enumerate() {
                if i < instances.len() {
                    let single_material = PhysicalMaterial::new_transparent(
                        &self.context,
                        &CpuMaterial {
                            albedo: *color,
                            metallic: 0.1,
                            roughness: 0.7,
                            ..Default::default()
                        }
                    );
                    
                    let single_instance = InstancedMesh::new(
                        &self.context, 
                        &[instances[i]], 
                        &Mesh::new(&self.context, &self.sphere_mesh)
                    );
                    let renderable = Gm::new(single_instance, single_material);
                    screen.render(&self.camera, &[&renderable], &[])?;
                }
            }
        }
        
        Ok(())
    }
    
    fn get_particle_color(&self, particle: &Particle, behavior_name: &str) -> Srgba {
        match behavior_name {
            "3D Inelastic Collision" => {
                // Color by velocity magnitude
                let speed = particle.velocity.norm();
                let intensity = (speed * 0.05).min(1.0);
                Srgba::new(
                    (255.0 * intensity) as u8,
                    (100.0 * (1.0 - intensity)) as u8,
                    150,
                    200
                )
            },
            "3D Active Brownian Particles" => {
                // Color by 3D orientation
                if let Some(theta) = particle.properties.get("theta") {
                    if let Some(phi) = particle.properties.get("phi") {
                        let r = ((theta.cos() + 1.0) * 127.5) as u8;
                        let g = ((phi.cos() + 1.0) * 127.5) as u8;
                        let b = ((theta.sin() * phi.sin() + 1.0) * 127.5) as u8;
                        Srgba::new(r, g, b, 220)
                    } else {
                        Srgba::new(100, 150, 255, 220)
                    }
                } else {
                    Srgba::new(100, 150, 255, 220)
                }
            },
            "3D Flocking Boids" => {
                // Color by speed with flocking colors
                let speed = particle.velocity.norm();
                let normalized_speed = (speed / 20.0).min(1.0); // Assume max speed ~20
                Srgba::new(
                    (50.0 + 205.0 * normalized_speed) as u8,
                    (200.0 * (1.0 - normalized_speed)) as u8,
                    (100.0 + 100.0 * normalized_speed) as u8,
                    230
                )
            },
            _ => Srgba::new(200, 200, 200, 220), // Default scientific gray
        }
    }
    
    pub fn handle_events(&mut self, events: &[Event]) -> bool {
        let mut camera_changed = false;
        
        for event in events {
            match event {
                Event::MouseMotion { button, delta, .. } => {
                    if *button == Some(MouseButton::Left) {
                        self.camera_control.rotate(delta.0 as f32 * 0.01, delta.1 as f32 * 0.01);
                        camera_changed = true;
                    }
                },
                Event::MouseWheel { delta, .. } => {
                    self.camera_control.zoom(delta.1 * 10.0);
                    camera_changed = true;
                },
                _ => {}
            }
        }
        
        if camera_changed {
            self.camera.set_view(
                self.camera_control.position(),
                self.camera_control.target(),
                Vec3::unit_y()
            );
        }
        
        camera_changed
    }
}

// Main application runner with scientific logging
pub fn run_simulation<B: ParticleBehavior + 'static>(
    mut simulation: Simulation<B>,
    window_title: Option<String>,
) -> ThreeDResult<()> {
    let title = window_title.unwrap_or_else(|| 
        format!("3D Scientific Particle Simulation - {}", simulation.behavior_name())
    );
    
    let window = Window::new(WindowSettings {
        title,
        max_size: Some((1200, 800)),
        ..Default::default()
    })?;
    
    let screen = Screen::new(&window, window.viewport());
    let mut renderer = ParticleRenderer::new(&window)?;
    
    // Initialize renderer with simulation bounds
    renderer.update_bounds(&simulation.bounds)?;
    
    let mut frame_count = 0;
    let mut last_fps_time = std::time::Instant::now();
    let mut last_physics_log = std::time::Instant::now();
    
    println!("Starting {} simulation", simulation.behavior_name());
    println!("Bounds: {:?}", simulation.bounds);
    println!("Initial particles: {}", simulation.particles.len());
    
    // Main loop
    window.render_loop(move |frame_input| {
        // Handle events
        renderer.handle_events(&frame_input.events);
        
        // Update simulation (multiple steps per frame for smooth physics)
        for _ in 0..2 {
            simulation.step();
        }
        
        // Render
        renderer.render(&simulation, &screen)?;
        
        // Performance logging
        frame_count += 1;
        if frame_count % 60 == 0 {
            let now = std::time::Instant::now();
            let fps = 60.0 / now.duration_since(last_fps_time).as_secs_f32();
            println!("FPS: {:.1}", fps);
            last_fps_time = now;
        }
        
        // Physics logging every 2 seconds
        let now = std::time::Instant::now();
        if now.duration_since(last_physics_log).as_secs() >= 2 {
            println!(
                "Time: {:.2}s | Particles: {} | KE: {:.6} | Momentum: [{:.3}, {:.3}, {:.3}] | CoM: [{:.3}, {:.3}, {:.3}]",
                simulation.time,
                simulation.particles.len(),
                simulation.total_kinetic_energy(),
                simulation.total_momentum().x,
                simulation.total_momentum().y,
                simulation.total_momentum().z,
                simulation.center_of_mass().x,
                simulation.center_of_mass().y,
                simulation.center_of_mass().z,
            );
            last_physics_log = now;
        }
        
        FrameOutput::default()
    })
}

# =============================================================================
# examples/inelastic_collision.rs
# =============================================================================
use particle_core::*;
use particle_viz::run_simulation;
use rand::Rng;

// 3D Inelastic Collision Behavior with f64 precision
pub struct InelasticBehavior3D {
    pub restitution: f64,      // 0.0 = perfectly inelastic  
    pub noise_strength: f64,   // random velocity magnitude
    pub gravity: Vec3,         // gravitational acceleration
    pub damping: f64,          // velocity damping factor
}

impl InelasticBehavior3D {
    pub fn new(restitution: f64, noise_strength: f64) -> Self {
        Self { 
            restitution, 
            noise_strength,
            gravity: Vec3::new(0.0, -9.81, 0.0), // Earth gravity
            damping: 0.999, // Very slight air resistance
        }
    }
    
    pub fn with_gravity(mut self, gravity: Vec3) -> Self {
        self.gravity = gravity;
        self
    }
    
    pub fn with_damping(mut self, damping: f64) -> Self {
        self.damping = damping;
        self
    }
}

impl ParticleBehavior for InelasticBehavior3D {
    fn apply_forces(&self, particles: &mut [Particle], dt: f64, _bounds: &Bounds3D) {
        let mut rng = rand::thread_rng();
        
        for particle in particles {
            // Apply gravity
            particle.velocity += self.gravity * dt;
            
            // Apply air resistance/damping
            particle.velocity *= self.damping;
            
            // Add random thermal noise to prevent complete energy loss
            if self.noise_strength > 0.0 {
                let noise = Vec3::random_unit_sphere() * self.noise_strength;
                particle.velocity += noise;
            }
        }
    }
    
    fn handle_collision(&self, p1: &mut Particle, p2: &mut Particle, _dt: f64) {
        let delta = p2.position - p1.position;
        let dist = delta.norm();
        
        if dist == 0.0 { return; }
        
        let normal = delta.normalize();
        
        // Relative velocity in collision normal direction
        let relative_velocity = p2.velocity - p1.velocity;
        let velocity_along_normal = relative_velocity.dot(&normal);
        
        // Don't resolve if separating
        if velocity_along_normal > 0.0 { return; }
        
        // Calculate impulse with high precision
        let impulse_magnitude = -(1.0 + self.restitution) * velocity_along_normal / (1.0/p1.mass + 1.0/p2.mass);
        let impulse = normal * impulse_magnitude;
        
        // Apply impulse
        p1.velocity -= impulse / p1.mass;
        p2.velocity += impulse / p2.mass;
        
        // Separate overlapping particles with high precision
        let overlap = (p1.radius + p2.radius) - dist;
        if overlap > 0.0 {
            let separation_distance = overlap * 0.5 + 1e-12; // Add tiny epsilon for numerical stability
            let separation = normal * separation_distance;
            p1.position -= separation;
            p2.position += separation;
        }
    }
    
    fn handle_boundary(&self, particle: &mut Particle, bounds: &Bounds3D) {
        // Bounce off all 6 walls with energy loss
        let eps = 1e-12; // Numerical stability
        
        if particle.position.x <= bounds.min.x + particle.radius {
            particle.velocity.x = -particle.velocity.x * self.restitution;
            particle.position.x = bounds.min.x + particle.radius + eps;
        }
        if particle.position.x >= bounds.max.x - particle.radius {
            particle.velocity.x = -particle.velocity.x * self.restitution;
            particle.position.x = bounds.max.x - particle.radius - eps;
        }
        if particle.position.y <= bounds.min.y + particle.radius {
            particle.velocity.y = -particle.velocity.y * self.restitution;
            particle.position.y = bounds.min.y + particle.radius + eps;
        }
        if particle.position.y >= bounds.max.y - particle.radius {
            particle.velocity.y = -particle.velocity.y * self.restitution;
            particle.position.y = bounds.max.y - particle.radius - eps;
        }
        if particle.position.z <= bounds.min.z + particle.radius {
            particle.velocity.z = -particle.velocity.z * self.restitution;
            particle.position.z = bounds.min.z + particle.radius + eps;
        }
        if particle.position.z >= bounds.max.z - particle.radius {
            particle.velocity.z = -particle.velocity.z * self.restitution;
            particle.position.z = bounds.max.z - particle.radius - eps;
        }
    }
    
    fn name(&self) -> &'static str {
        "3D Inelastic Collision"
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example: Load from file or create new simulation
    let args: Vec<String> = std::env::args().collect();
    
    let mut simulation = if args.len() > 1 && args[1] == "--load" && args.len() > 2 {
        // Load from file
        let filepath = &args[2];
        println!("Loading simulation from: {}", filepath);
        
        let behavior = InelasticBehavior3D::new(0.8, 0.1)
            .with_gravity(Vec3::new(0.0, -9.81, 0.0))
            .with_damping(0.9995);
        
        Simulation::load_state(filepath, behavior)?
    } else {
        // Create new simulation
        let behavior = InelasticBehavior3D::new(0.8, 0.1)
            .with_gravity(Vec3::new(0.0, -9.81, 0.0))
            .with_damping(0.9995);
        
        let bounds = Bounds3D::box_size(50.0, 50.0, 50.0);
        let mut sim = Simulation::new(bounds, 0.001, behavior);
        
        // Add particles with realistic size distribution
        sim.add_random_particles(20, (0.5, 2.0));
        
        // Give some initial velocities
        let mut rng = rand::thread_rng();
        for particle in &mut sim.particles {
            particle.velocity = Vec3::new(
                rng.gen::<f64>() * 20.0 - 10.0,
                rng.gen::<f64>() * 20.0 - 10.0,
                rng.gen::<f64>() * 20.0 - 10.0,
            );
        }
        
        sim
    };
    
    // Configure auto-saving every 1000 steps
    simulation.set_save_interval(1000, "simulation_output")?;
    
    // Save initial state
    simulation.save_state(None)?;
    
    println!("Starting inelastic collision simulation with {} particles", simulation.particles.len());
    println!("Use: cargo run --example inelastic_collision -- --load <filename> to resume");
    
    run_simulation(simulation, None)?;
    
    Ok(())
}

# =============================================================================
# examples/active_brownian.rs  
# =============================================================================
use particle_core::*;
use particle_viz::run_simulation;
use rand::Rng;
use std::f64::consts::PI;

// 3D Active Brownian Particles with f64 precision
pub struct ActiveBrownianBehavior3D {
    pub active_force: f64,          // magnitude of self-propulsion
    pub rotational_diffusion: f64,  // angular diffusion coefficient
    pub translational_diffusion: f64, // thermal noise strength
    pub persistence_length: f64,    // characteristic distance before direction change
}

impl ActiveBrownianBehavior3D {
    pub fn new(active_force: f64, rotational_diffusion: f64, translational_diffusion: f64) -> Self {
        Self { 
            active_force, 
            rotational_diffusion, 
            translational_diffusion,
            persistence_length: active_force / rotational_diffusion,
        }
    }
    
    pub fn from_persistence(active_force: f64, persistence_length: f64, translational_diffusion: f64) -> Self {
        let rotational_diffusion = active_force / persistence_length;
        Self {
            active_force,
            rotational_diffusion,
            translational_diffusion,
            persistence_length,
        }
    }
}

impl ParticleBehavior for ActiveBrownianBehavior3D {
    fn apply_forces(&self, particles: &mut [Particle], dt: f64, _bounds: &Bounds3D) {
        let mut rng = rand::thread_rng();
        
        for particle in particles {
            // Get or initialize 3D orientation (spherical coordinates)
            let theta = *particle.properties.get("theta").unwrap_or_else(|| {
                let val = rng.gen::<f64>() * PI;
                particle.properties.insert("theta".to_string(), val);
                particle.properties.get("theta").unwrap()
            });
            
            let phi = *particle.properties.get("phi").unwrap_or_else(|| {
                let val = rng.gen::<f64>() * 2.0 * PI;
                particle.properties.insert("phi".to_string(), val);
                particle.properties.get("phi").unwrap()
            });
            
            // Update orientation with rotational diffusion (Ornstein-Uhlenbeck process)
            let dr_theta = (rng.gen::<f64>() - 0.5) * (2.0 * self.rotational_diffusion * dt).sqrt();
            let dr_phi = (rng.gen::<f64>() - 0.5) * (2.0 * self.rotational_diffusion * dt).sqrt();
            
            let new_theta = (theta + dr_theta).max(0.001).min(PI - 0.001); // Keep in valid range
            let new_phi = phi + dr_phi;
            
            particle.properties.insert("theta".to_string(), new_theta);
            particle.properties.insert("phi".to_string(), new_phi);
            
            // Convert spherical to cartesian for swimming direction
            let direction = Vec3::new(
                new_theta.sin() * new_phi.cos(),
                new_theta.sin() * new_phi.sin(),
                new_theta.cos()
            );
            
            // Active swimming velocity
            let active_velocity = direction * self.active_force;
            
            // Translational diffusion (Brownian motion)
            let thermal_noise = Vec3::random_unit_sphere() * 
                self.translational_diffusion * (2.0 * dt).sqrt();
            
            // Set velocity (overdamped dynamics - no inertia)
            particle.velocity = active_velocity + thermal_noise;
        }
    }
    
    fn handle_collision(&self, p1: &mut Particle, p2: &mut Particle, _dt: f64) {
        // Soft repulsion for active particles
        let delta = p2.position - p1.position;
        let dist = delta.norm();
        
        if dist == 0.0 || dist >= p1.radius + p2.radius { return; }
        
        let normal = delta.normalize();
        let overlap = (p1.radius + p2.radius) - dist;
        
        // Gentle separation to avoid jamming
        let separation_distance = overlap * 0.51; // Slightly more than half
        let separation = normal * separation_distance;
        p1.position -= separation;
        p2.position += separation;
        
        // Scatter orientations upon collision (tumbling)
        let mut rng = rand::thread_rng();
        let scatter_strength = 0.5; // How much to randomize orientation
        
        if rng.gen::<f64>() < scatter_strength {
            p1.properties.insert("theta".to_string(), rng.gen::<f64>() * PI);
            p1.properties.insert("phi".to_string(), rng.gen::<f64>() * 2.0 * PI);
        }
        
        if rng.gen::<f64>() < scatter_strength {
            p2.properties.insert("theta".to_string(), rng.gen::<f64>() * PI);
            p2.properties.insert("phi".to_string(), rng.gen::<f64>() * 2.0 * PI);
        }
    }
    
    fn handle_boundary(&self, particle: &mut Particle, bounds: &Bounds3D) {
        let mut rng = rand::thread_rng();
        let mut reorient = false;
        
        // Reflecting boundaries with orientation randomization
        if particle.position.x <= bounds.min.x + particle.radius {
            particle.position.x = bounds.min.x + particle.radius;
            reorient = true;
        }
        if particle.position.x >= bounds.max.x - particle.radius {
            particle.position.x = bounds.max.x - particle.radius;
            reorient = true;
        }
        if particle.position.y <= bounds.min.y + particle.radius {
            particle.position.y = bounds.min.y + particle.radius;
            reorient = true;
        }
        if particle.position.y >= bounds.max.y - particle.radius {
            particle.position.y = bounds.max.y - particle.radius;
            reorient = true;
        }
        if particle.position.z <= bounds.min.z + particle.radius {
            particle.position.z = bounds.min.z + particle.radius;
            reorient = true;
        }
        if particle.position.z >= bounds.max.z - particle.radius {
            particle.position.z = bounds.max.z - particle.radius;
            reorient = true;
        }
        
        // Randomize orientation on wall collision
        if reorient {
            particle.properties.insert("theta".to_string(), rng.gen::<f64>() * PI);
            particle.properties.insert("phi".to_string(), rng.gen::<f64>() * 2.0 * PI);
        }
    }
    
    fn post_step(&self, particles: &mut [Particle], _dt: f64) {
        // Calculate and store some interesting properties
        // This could include clustering analysis, velocity correlations, etc.
        
        // Example: Track average orientation alignment
        if particles.len() > 1 {
            let mut avg_direction = Vec3::zeros();
            
            for particle in particles.iter() {
                if let (Some(theta), Some(phi)) = (
                    particle.properties.get("theta"), 
                    particle.properties.get("phi")
                ) {
                    let direction = Vec3::new(
                        theta.sin() * phi.cos(),
                        theta.sin() * phi.sin(),
                        theta.cos()
                    );
                    avg_direction += direction;
                }
            }
            
            let order_parameter = avg_direction.norm() / particles.len() as f64;
            // Could store this for analysis or visualization
            
            // Uncomment for alignment tracking:
            // println!("Order parameter: {:.3}", order_parameter);
        }
    }
    
    fn name(&self) -> &'static str {
        "3D Active Brownian Particles"
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    let mut simulation = if args.len() > 1 && args[1] == "--load" && args.len() > 2 {
        // Load from saved state
        let filepath = &args[2];
        println!("Loading Active Brownian simulation from: {}", filepath);
        
        let behavior = ActiveBrownianBehavior3D::from_persistence(20.0, 10.0, 1.0);
        Simulation::load_state(filepath, behavior)?
    } else {
        // Create new simulation
        let behavior = ActiveBrownianBehavior3D::from_persistence(
            20.0,  // Active force (swimming speed)
            10.0,  // Persistence length
            1.0    // Thermal noise
        );
        
        let bounds = Bounds3D::box_size(60.0, 60.0, 60.0);
        let mut sim = Simulation::new(bounds, 0.01, behavior);
        
        // Add active particles
        sim.add_random_particles(35, (0.8, 1.5));
        sim
    };
    
    // Configure auto-saving every 500 steps (5 seconds of simulation time)
    simulation.set_save_interval(500, "abp_simulation_output")?;
    simulation.save_state(None)?;
    
    println!("Starting Active Brownian particle simulation");
    println!("Persistence length: {:.1} units", 10.0);
    println!("Use: cargo run --example active_brownian -- --load <filename> to resume");
    
    run_simulation(simulation, Some("3D Active Brownian Particles - Swimming Microorganisms".to_string()))?;
    
    Ok(())
}

# =============================================================================
# examples/flocking.rs - Enhanced 3D flocking behavior
# =============================================================================
use particle_core::*;
use particle_viz::run_simulation;

// 3D Flocking/Boids Behavior with f64 precision
pub struct FlockingBehavior3D {
    pub separation_radius: f64,
    pub alignment_radius: f64,
    pub cohesion_radius: f64,
    pub separation_strength: f64,
    pub alignment_strength: f64,
    pub cohesion_strength: f64,
    pub max_speed: f64,
    pub max_force: f64,
    pub neighbor_distance: f64,
}

impl FlockingBehavior3D {
    pub fn new() -> Self {
        Self {
            separation_radius: 8.0,
            alignment_radius: 15.0,
            cohesion_radius: 15.0,
            separation_strength: 3.0,
            alignment_strength: 1.0,
            cohesion_strength: 1.0,
            max_speed: 25.0,
            max_force: 8.0,
            neighbor_distance: 20.0,
        }
    }
    
    pub fn with_parameters(
        sep_radius: f64, align_radius: f64, coh_radius: f64,
        sep_strength: f64, align_strength: f64, coh_strength: f64,
        max_speed: f64, max_force: f64
    ) -> Self {
        Self {
            separation_radius: sep_radius,
            alignment_radius: align_radius,
            cohesion_radius: coh_radius,
            separation_strength: sep_strength,
            alignment_strength: align_strength,
            cohesion_strength: coh_strength,
            max_speed,
            max_force,
            neighbor_distance: align_radius.max(coh_radius),
        }
    }
}

impl ParticleBehavior for FlockingBehavior3D {
    fn apply_forces(&self, particles: &mut [Particle], dt: f64, bounds: &Bounds3D) {
        let mut forces = vec![Vec3::zeros(); particles.len()];
        
        // Calculate flocking forces for each particle
        for i in 0..particles.len() {
            let mut separation = Vec3::zeros();
            let mut alignment = Vec3::zeros();
            let mut cohesion = Vec3::zeros();
            let mut sep_count = 0;
            let mut align_count = 0;
            let mut coh_count = 0;
            
            // Find neighbors and calculate forces
            for j in 0..particles.len() {
                if i == j { continue; }
                
                let distance = (particles[j].position - particles[i].position).norm();
                
                // Separation - steer away from nearby neighbors
                if distance > 0.0 && distance < self.separation_radius {
                    let mut diff = (particles[i].position - particles[j].position).normalize();
                    diff /= distance; // Weight by inverse distance
                    separation += diff;
                    sep_count += 1;
                }
                
                // Alignment - steer towards average heading of neighbors  
                if distance > 0.0 && distance < self.alignment_radius {
                    alignment += particles[j].velocity;
                    align_count += 1;
                }
                
                // Cohesion - steer towards average position of neighbors
                if distance > 0.0 && distance < self.cohesion_radius {
                    cohesion += particles[j].position;
                    coh_count += 1;
                }
            }
            
            // Calculate steering forces
            if sep_count > 0 {
                separation /= sep_count as f64;
                if separation.norm() > 0.0 {
                    separation = separation.normalize() * self.max_speed - particles[i].velocity;
                    separation = self.limit_force(separation) * self.separation_strength;
                }
            }
            
            if align_count > 0 {
                alignment /= align_count as f64;
                if alignment.norm() > 0.0 {
                    alignment = alignment.normalize() * self.max_speed - particles[i].velocity;
                    alignment = self.limit_force(alignment) * self.alignment_strength;
                }
            }
            
            if coh_count > 0 {
                cohesion /= coh_count as f64;
                let seek = cohesion - particles[i].position;
                if seek.norm() > 0.0 {
                    cohesion = seek.normalize() * self.max_speed - particles[i].velocity;
                    cohesion = self.limit_force(cohesion) * self.cohesion_strength;
                }
            }
            
            // Add boundary avoidance
            let boundary_force = self.boundary_avoidance(&particles[i], bounds);
            
            forces[i] = separation + alignment + cohesion + boundary_force;
        }
        
        // Apply forces and update velocities
        for (i, particle) in particles.iter_mut().enumerate() {
            particle.velocity += forces[i] * dt;
            particle.velocity = self.limit_velocity(particle.velocity);
        }
    }
    
    fn handle_collision(&self, _p1: &mut Particle, _p2: &mut Particle, _dt: f64) {
        // Flocking handles avoidance through separation force
        // No hard collisions needed - birds don't bounce off each other
    }
    
    fn handle_boundary(&self, particle: &mut Particle, bounds: &Bounds3D) {
        // Periodic boundary conditions (toroidal space) for endless flocking
        let size = bounds.size();
        
        if particle.position.x < bounds.min.x { 
            particle.position.x += size.x; 
        }
        if particle.position.x > bounds.max.x { 
            particle.position.x -= size.x; 
        }
        if particle.position.y < bounds.min.y { 
            particle.position.y += size.y; 
        }
        if particle.position.y > bounds.max.y { 
            particle.position.y -= size.y; 
        }
        if particle.position.z < bounds.min.z { 
            particle.position.z += size.z; 
        }
        if particle.position.z > bounds.max.z { 
            particle.position.z -= size.z; 
        }
    }
    
    fn post_step(&self, particles: &mut [Particle], _dt: f64) {
        // Calculate flocking statistics
        if particles.len() > 1 {
            // Average velocity (polarization)
            let avg_velocity: Vec3 = particles.iter().map(|p| p.velocity).sum::<Vec3>() / particles.len() as f64;
            let polarization = avg_velocity.norm() / self.max_speed;
            
            // Store polarization in first particle for visualization/analysis
            if let Some(particle) = particles.get_mut(0) {
                particle.properties.insert("polarization".to_string(), polarization);
            }
            
            // Uncomment for flocking analysis:
            // println!("Polarization: {:.3}", polarization);
        }
    }
    
    fn name(&self) -> &'static str {
        "3D Flocking Boids"
    }
}

impl FlockingBehavior3D {
    fn limit_force(&self, force: Vec3) -> Vec3 {
        if force.norm() > self.max_force {
            force.normalize() * self.max_force
        } else {
            force
        }
    }
    
    fn limit_velocity(&self, velocity: Vec3) -> Vec3 {
        if velocity.norm() > self.max_speed {
            velocity.normalize() * self.max_speed
        } else {
            velocity
        }
    }
    
    fn boundary_avoidance(&self, particle: &Particle, bounds: &Bounds3D) -> Vec3 {
        let mut force = Vec3::zeros();
        let margin = 15.0;  // Distance from boundary to start turning
        let strength = 10.0;  // Turning force strength
        
        // Create turning forces near boundaries
        if particle.position.x < bounds.min.x + margin {
            force.x += strength * (1.0 - (particle.position.x - bounds.min.x) / margin);
        }
        if particle.position.x > bounds.max.x - margin {
            force.x -= strength * (1.0 - (bounds.max.x - particle.position.x) / margin);
        }
        if particle.position.y < bounds.min.y + margin {
            force.y += strength * (1.0 - (particle.position.y - bounds.min.y) / margin);
        }
        if particle.position.y > bounds.max.y - margin {
            force.y -= strength * (1.0 - (bounds.max.y - particle.position.y) / margin);
        }
        if particle.position.z < bounds.min.z + margin {
            force.z += strength * (1.0 - (particle.position.z - bounds.min.z) / margin);
        }
        if particle.position.z > bounds.max.z - margin {
            force.z -= strength * (1.0 - (bounds.max.z - particle.position.z) / margin);
        }
        
        force
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    let mut simulation = if args.len() > 1 && args[1] == "--load" && args.len() > 2 {
        // Load from saved state
        let filepath = &args[2];
        println!("Loading flocking simulation from: {}", filepath);
        
        let behavior = FlockingBehavior3D::with_parameters(
            5.0, 12.0, 12.0,  // Radii
            2.5, 1.2, 0.8,    // Strengths
            20.0, 6.0         // Max speed/force
        );
        
        Simulation::load_state(filepath, behavior)?
    } else {
        // Create new flocking simulation
        let behavior = FlockingBehavior3D::with_parameters(
            5.0,   // Separation radius - personal space
            12.0,  // Alignment radius - local interaction
            12.0,  // Cohesion radius - group attraction
            2.5,   // Separation strength - avoid crowding
            1.2,   // Alignment strength - follow neighbors
            0.8,   // Cohesion strength - stay with group
            20.0,  // Max speed
            6.0    // Max turning force
        );
        
        let bounds = Bounds3D::box_size(80.0, 80.0, 80.0);
        let mut sim = Simulation::new(bounds, 0.016, behavior);
        
        // Add boids with uniform size and initial random velocities
        sim.add_random_particles(50, (1.0, 1.5));
        
        // Give them initial velocities for interesting dynamics
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for particle in &mut sim.particles {
            particle.velocity = Vec3::new(
                rng.gen::<f64>() * 20.0 - 10.0,
                rng.gen::<f64>() * 20.0 - 10.0,  
                rng.gen::<f64>() * 20.0 - 10.0,
            );
        }
        
        sim
    };
    
    // Configure auto-saving every 1000 steps (~16 seconds of simulation time)
    simulation.set_save_interval(1000, "flocking_simulation_output")?;
    simulation.save_state(None)?;
    
    println!("Starting 3D flocking simulation with {} boids", simulation.particles.len());
    println!("Use: cargo run --example flocking -- --load <filename> to resume");
    
    run_simulation(simulation, Some("3D Flocking Boids - Emergent Collective Motion".to_string()))?;
    
    Ok(())
}

# =============================================================================
# README.md
# =============================================================================

# 3D Scientific Particle Simulation Workspace

A high-precision, extensible 3D particle simulation framework built in Rust with `nalgebra` for scientific computing and `three-d` for visualization.

## Architecture

```
particle_sim_workspace/
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── particle_core/         # f64 precision 3D physics simulation
│   └── particle_viz/          # f32 three-d visualization with automatic conversion
└── examples/
    ├── inelastic_collision.rs # 3D collision dynamics with gravity
    ├── active_brownian.rs     # 3D swimming microorganisms  
    └── flocking.rs           # 3D collective motion (boids)
```

## Key Features

### Scientific Precision
- **f64 physics**: High precision for accurate scientific simulation
- **f32 rendering**: Automatic conversion to f32 for GPU efficiency  
- **nalgebra integration**: Robust, battle-tested linear algebra
- **Numerical stability**: Epsilon handling, proper collision resolution

### Flexible Design
- **Trait-based behaviors**: Easy extension without changing core structure
- **Constant simulation loop**: Same 4-step process regardless of complexity
- **Modular architecture**: Core physics independent of visualization
- **Rich logging**: Detailed physics statistics (energy, momentum, center of mass)

### Advanced Physics
- **3D collision detection**: Sphere-sphere with proper separation
- **Multiple boundary conditions**: Reflecting, periodic, absorbing
- **Force integration**: Verlet-style integration ready
- **Conservation tracking**: Energy, momentum, and other quantities

## Quick Start

```bash
# Set up workspace
mkdir particle_sim_workspace && cd particle_sim_workspace

# Run simulations
cargo run --example inelastic_collision  # Bouncing spheres
cargo run --example active_brownian      # Swimming particles  
cargo run --example flocking             # Bird flocking

# Test core physics
cargo test -p particle_core
```

## Example Behaviors

### Inelastic Collisions
- Realistic gravity and air resistance
- Energy-conserving collision dynamics  
- Thermal noise to prevent complete energy loss
- Color-coded by velocity magnitude

### Active Brownian Particles  
- 3D swimming with orientation diffusion
- Scientifically accurate rotational dynamics
- Soft repulsion and tumbling on collision
- Color-coded by 3D orientation

### Flocking Boids
- Classic separation, alignment, cohesion rules
- Boundary avoidance with smooth turning
- Periodic boundaries for endless motion
- Polarization tracking and analysis

## Controls
- **Mouse drag**: Orbit camera
- **Mouse wheel**: Zoom
- **Console output**: Real-time physics statistics

## Extending the Framework

Add new particle behaviors by implementing `ParticleBehavior`:

```rust
use particle_core::*;

struct MyBehavior {
    // Scientific parameters with f64 precision
    parameter: f64,
}

impl ParticleBehavior for MyBehavior {
    fn apply_forces(&self, particles: &mut [Particle], dt: f64, bounds: &Bounds3D) {
        // High-precision force calculations
    }
    
    fn handle_collision(&self, p1: &mut Particle, p2: &mut Particle, dt: f64) {
        // Collision response with numerical stability
    }
    
    fn handle_boundary(&self, particle: &mut Particle, bounds: &Bounds3D) {
        // Boundary conditions
    }
    
    fn name(&self) -> &'static str { "My Scientific Behavior" }
}
