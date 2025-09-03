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
