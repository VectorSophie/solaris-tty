//! The simulation world: owns bodies and steps them forward.

use super::body::{vec_len, vec_sub, Body};
use super::diagnostics;
use super::gravity::accelerations;
use super::integrator::leapfrog_step;

/// Record of a resolved collision, for the trace panel.
pub struct Collision {
    pub survivor_name: String,
    pub other_name: String,
    pub m_survivor: f64,
    pub m_other: f64,
    pub v_rel: f64,
    pub merged_mass: f64,
    pub merged_speed: f64,
    /// Index of the surviving body after the removal.
    pub survivor: usize,
    /// Index that was removed from the bodies vec.
    pub removed: usize,
}

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

    /// Find the first overlapping pair (real radii touch) and merge it into a
    /// single body via a momentum-conserving perfectly-inelastic collision.
    /// Returns a record of the event, or None if nothing collided. Call in a
    /// loop to resolve all collisions in a frame.
    pub fn resolve_one_collision(&mut self) -> Option<Collision> {
        let n = self.bodies.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let d = vec_len(vec_sub(self.bodies[i].pos, self.bodies[j].pos));
                if d < self.bodies[i].radius + self.bodies[j].radius {
                    return Some(self.merge(i, j));
                }
            }
        }
        None
    }

    /// Merge bodies `i` and `j`; the more massive keeps its identity. Removes
    /// the other, recomputes cached state, and returns the collision record.
    fn merge(&mut self, i: usize, j: usize) -> Collision {
        let (keep, drop) = if self.bodies[i].mass >= self.bodies[j].mass {
            (i, j)
        } else {
            (j, i)
        };
        let a = self.bodies[keep].clone();
        let b = self.bodies[drop].clone();
        let m = a.mass + b.mass;
        let v_rel = vec_len(vec_sub(a.vel, b.vel));
        let com = |ax: [f64; 3], bx: [f64; 3]| {
            [
                (a.mass * ax[0] + b.mass * bx[0]) / m,
                (a.mass * ax[1] + b.mass * bx[1]) / m,
                (a.mass * ax[2] + b.mass * bx[2]) / m,
            ]
        };
        let vel = com(a.vel, b.vel);
        let pos = com(a.pos, b.pos);
        // Volume-preserving radius keeps density sensible.
        let radius = (a.radius.powi(3) + b.radius.powi(3)).cbrt();

        {
            let survivor = &mut self.bodies[keep];
            survivor.mass = m;
            survivor.pos = pos;
            survivor.vel = vel;
            survivor.radius = radius;
            survivor.trail.clear();
        }
        self.bodies.remove(drop);

        // Recompute cached acceleration (length must track body count).
        self.acc = accelerations(&self.bodies, self.g, self.softening);
        self.energy_ref = self.total_energy();

        let survivor_idx = if drop < keep { keep - 1 } else { keep };
        Collision {
            survivor_name: a.name,
            other_name: b.name,
            m_survivor: a.mass,
            m_other: b.mass,
            v_rel,
            merged_mass: m,
            merged_speed: vec_len(vel),
            survivor: survivor_idx,
            removed: drop,
        }
    }

    /// Add a body at runtime, recomputing cached acceleration (its length must
    /// track the body count) and re-baselining the reference energy.
    pub fn add_body(&mut self, body: Body) -> usize {
        self.bodies.push(body);
        self.refresh_forces();
        self.bodies.len() - 1
    }

    /// Recompute cached acceleration and re-baseline reference energy after an
    /// in-place edit to a body's mass/position (e.g. via `:set`).
    pub fn refresh_forces(&mut self) {
        self.acc = accelerations(&self.bodies, self.g, self.softening);
        self.energy_ref = self.total_energy();
    }
}
