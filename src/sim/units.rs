//! Physical constants and unit helpers. Everything in the sim is f64, SI
//! (metres, kilograms, seconds).

/// Newtonian gravitational constant (CODATA 2018), m^3 kg^-1 s^-2.
pub const G: f64 = 6.674_30e-11;

/// Astronomical unit (IAU 2012), metres.
pub const AU: f64 = 1.495_978_707e11;

/// Standard gravitational parameter of the Sun, m^3 s^-2 (IAU).
pub const GM_SUN: f64 = 1.327_124_400_18e20;

/// Mass of the Sun, kg.
pub const M_SUN: f64 = GM_SUN / G;
