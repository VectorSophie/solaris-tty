//! The simulation world: owns bodies and steps them forward.

use super::body::Body;
use super::diagnostics;
use super::gravity::accelerations;
use super::integrator::leapfrog_step;

pub struct World {
    pub bodies: Vec<Body>,
    pub g: f64,
    pub dt: f64,          // seconds per step
    pub substeps: u32,    // leapfrog steps per advance()
    pub softening: f64,   // m
    pub time: f64,        // elapsed sim seconds
    /// Reference energy captured at construction, for drift reporting.
    pub energy_ref: f64,
    /// Cached acceleration at current positions, reused across steps.
    acc: Vec<[f64; 3]>,
}

impl World {
    pub fn new(bodies: Vec<Body>, g: f64, dt: f64, substeps: u32, softening: f64) -> Self {
        let acc = accelerations(&bodies, g, softening);
        let energy_ref = diagnostics::total_energy(&bodies, g);
        World {
            bodies,
            g,
            dt,
            substeps: substeps.max(1),
            softening,
            time: 0.0,
            energy_ref,
            acc,
        }
    }

    /// Remove the net drift of the system's centre of mass so the barycentre
    /// stays put: V_com = Σmᵢvᵢ / Σmᵢ ; vᵢ' = vᵢ − V_com.
    /// Returns V_com (for the load trace). Re-baselines the reference energy.
    pub fn apply_barycentric_correction(&mut self) -> [f64; 3] {
        let total_mass: f64 = self.bodies.iter().map(|b| b.mass).sum();
        if total_mass == 0.0 {
            return [0.0; 3];
        }
        let p = diagnostics::total_momentum(&self.bodies);
        let v_com = [p[0] / total_mass, p[1] / total_mass, p[2] / total_mass];
        for b in &mut self.bodies {
            for k in 0..3 {
                b.vel[k] -= v_com[k];
            }
        }
        self.acc = accelerations(&self.bodies, self.g, self.softening);
        self.energy_ref = diagnostics::total_energy(&self.bodies, self.g);
        v_com
    }

    /// Advance one rendered tick: `substeps` leapfrog steps of `dt`.
    pub fn advance(&mut self) {
        for _ in 0..self.substeps {
            self.acc = leapfrog_step(
                &mut self.bodies,
                &self.acc,
                self.dt,
                self.g,
                self.softening,
            );
            self.time += self.dt;
        }
    }

    /// Append current positions to every body's trail.
    pub fn record_trails(&mut self, max_len: usize) {
        for b in &mut self.bodies {
            b.push_trail(max_len);
        }
    }

    pub fn total_energy(&self) -> f64 {
        diagnostics::total_energy(&self.bodies, self.g)
    }

    pub fn energy_drift_pct(&self) -> f64 {
        diagnostics::energy_drift_pct(self.total_energy(), self.energy_ref)
    }

    pub fn find_body(&self, name: &str) -> Option<usize> {
        self.bodies.iter().position(|b| b.name == name)
    }
}
