//! Body orientation and rotation system using IAU/NAIF conventions

use glam::{Mat3, Vec3};

/// Uniform representation of body orientation following NAIF/IAU conventions
#[derive(Clone, Copy, Debug)]
pub struct UniformOrientation {
    /// Right Ascension of North Pole (degrees)
    pub pole_ra_deg: f64,
    /// Declination of North Pole (degrees)
    pub pole_dec_deg: f64,
    /// Prime Meridian angle at reference epoch (degrees)
    pub prime_meridian_deg: f64,
    /// Rotation rate (degrees per day)
    pub rotation_rate_deg_per_day: f64,
    /// Reference epoch (Julian day)
    pub epoch_jd: f64,
}

impl UniformOrientation {
    /// Create a default orientation suitable for planets without specific IAU data
    pub fn new(
        pole_ra_deg: f64,
        pole_dec_deg: f64,
        prime_meridian_deg: f64,
        rotation_rate_deg_per_day: f64,
        epoch_jd: f64,
    ) -> Self {
        Self {
            pole_ra_deg,
            pole_dec_deg,
            prime_meridian_deg,
            rotation_rate_deg_per_day,
            epoch_jd,
        }
    }

    /// Compute the body-fixed to inertial rotation matrix at given time
    pub fn rotation_matrix(&self, time: f64) -> Mat3 {
        let elapsed_days = time - self.epoch_jd;
        let rotation_angle = self.rotation_rate_deg_per_day * elapsed_days;

        // Apply NAIF/IAU rotation sequence:
        // R_body_from_eq = R3(W) * R1(90° - DEC) * R3(90° + RA)
        // where:
        // - W = prime meridian angle = prime_meridian_deg + rotation_angle
        // - DEC = pole_dec_deg
        // - RA = pole_ra_deg

        let w = (self.prime_meridian_deg + rotation_angle).to_radians();
        let dec = self.pole_dec_deg.to_radians();
        let ra = self.pole_ra_deg.to_radians();

        // R3(W) - rotation around z-axis
        let (w_sin, w_cos) = w.sin_cos();
        let r3_w = Mat3::from_cols_array(&[w_cos, w_sin, 0.0, -w_sin, w_cos, 0.0, 0.0, 0.0, 1.0]);

        // R1(90° - DEC) - rotation around x-axis
        let (dec_sin, dec_cos) = (90.0_f64.to_radians() - dec).sin_cos();
        let r1_dec =
            Mat3::from_cols_array(&[1.0, 0.0, 0.0, 0.0, dec_cos, dec_sin, 0.0, -dec_sin, dec_cos]);

        // R3(90° + RA) - rotation around z-axis
        let (ra_sin, ra_cos) = (90.0_f64.to_radians() + ra).sin_cos();
        let r3_ra =
            Mat3::from_cols_array(&[ra_cos, ra_sin, 0.0, -ra_sin, ra_cos, 0.0, 0.0, 0.0, 1.0]);

        r3_w * r1_dec * r3_ra
    }

    /// Get the body-fixed position vector at a specific time
    pub fn body_fixed_position(&self, time: f64, inertial_vector: Vec3) -> Vec3 {
        self.rotation_matrix(time).transpose() * inertial_vector
    }

    /// Get the body-fixed direction vector at a specific time
    pub fn body_fixed_direction(&self, time: f64, inertial_vector: Vec3) -> Vec3 {
        self.rotation_matrix(time) * inertial_vector
    }

    /// Compute body longitude from a direction vector (in body-fixed frame)
    pub fn longitude_from_vector(&self, time: f64, body_fixed_vector: Vec3) -> f64 {
        // Determine longitude from body-fixed vector - remember that longitude
        // is measured in the equatorial plane around the north pole
        let latitude = body_fixed_vector.y.atan2(
            (body_fixed_vector.x * body_fixed_vector.x + body_fixed_vector.z * body_fixed_vector.z)
                .sqrt(),
        );

        // Calculate longitude (angle in x-z plane relative to prime meridian)
        let longitude = body_fixed_vector.z.atan2(body_fixed_vector.x);

        // Convert to proper degrees and normalize to [0, 360)
        let mut result = longitude.to_degrees();
        if result < 0.0 {
            result += 360.0;
        }
        result
    }
}

// Earth orientation parameters (IAU 2000/2006)
pub const EARTH_ORIENTATION: UniformOrientation = UniformOrientation {
    pole_ra_deg: 0.0,
    pole_dec_deg: 90.0,
    prime_meridian_deg: 0.0, // Actually keeps track of Earth's rotation
    rotation_rate_deg_per_day: 360.985_684_232_3,
    epoch_jd: 2451545.0, // J2000 epoch
};

// Jupiter orientation parameters (IAU 2000/2006)
pub const JUPITER_ORIENTATION: UniformOrientation = UniformOrientation {
    pole_ra_deg: 268.056_53,
    pole_dec_deg: 64.495_38,
    prime_meridian_deg: 195.087_66, // Much better estimate
    rotation_rate_deg_per_day: 12.557_0,
    epoch_jd: 2451545.0,
};

// Basic fallback for bodies without explicit orientation
#[derive(Debug, Clone, Copy)]
pub struct ApproximateObliquity {
    pub axial_tilt: f64,     // degrees
    pub rotation_hours: f64, // hours
}

impl ApproximateObliquity {
    pub fn new(axial_tilt: f64, rotation_hours: f64) -> Self {
        Self {
            axial_tilt,
            rotation_hours,
        }
    }

    pub fn to_uniform_orientation(&self, epoch_jd: f64) -> UniformOrientation {
        // For planets with basic obliquity/rotation data
        // We'll make some reasonable approximations
        let rotation_rate = 360.0 / (self.rotation_hours * 3600.0);

        // For now, we'll let the pole points along the z-axis
        // and direction of rotation etc. for Earth, Jupiter etc.
        const EARTH_RA: f64 = 0.0; // We'll re-use the polar orientation
        const EARTH_DEC: f64 = 90.0;
        const JUPITER_RA: f64 = 268.05653;
        const JUPITER_DEC: f64 = 64.49538;

        // Give a meaningful default
        UniformOrientation {
            pole_ra_deg: JUPITER_RA,
            pole_dec_deg: JUPITER_DEC,
            prime_meridian_deg: 0.0, // Will be calculated based on current time
            rotation_rate_deg_per_day: rotation_rate,
            epoch_jd,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation_basic() {
        let mut orient = UniformOrientation::new(0.0, 90.0, 0.0, 360.0, 0.0);
        let rot1 = orient.rotation_matrix(0.0);
        let rot2 = orient.rotation_matrix(1.0);

        // Identity matrix at zero time
        assert_eq!(rot1, Mat3::IDENTITY);

        // Different rotation at 1 day
        assert!(rot2 != Mat3::IDENTITY);
    }

    #[test]
    fn test_earth_orientation() {
        assert_eq!(EARTH_ORIENTATION.pole_ra_deg, 0.0);
        assert_eq!(EARTH_ORIENTATION.pole_dec_deg, 90.0);
        assert_eq!(
            EARTH_ORIENTATION.rotation_rate_deg_per_day,
            360.985_684_232_3
        );
    }
}
