//! Planet surface rendering logic

use crossterm::style::Color;
use glam::Vec3;

use crate::render::scene::RenderQuality;
use crate::sim::body::Kind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceModel {
    Uniform,
    Earth,
    Jupiter,
    // Later: Mars, Moon, etc.
}

#[derive(Clone, Copy, Debug)]
pub struct SurfaceParams {
    pub detail: SurfaceDetail,
    pub quality: RenderQuality,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceDetail {
    None,
    Broad,
    Detailed,
}

pub fn albedo(model: SurfaceModel, latitude: f64, longitude: f64, params: SurfaceParams) -> Color {
    match model {
        SurfaceModel::Uniform => {
            // Basic default albedo for non-surface planets
            Color::Rgb {
                r: 128,
                g: 128,
                b: 128,
            }
        }
        SurfaceModel::Earth => earth_albedo(latitude, longitude, params.detail),
        SurfaceModel::Jupiter => jupiter_albedo(latitude, longitude, params.detail),
    }
}

fn earth_albedo(latitude: f64, longitude: f64, detail: SurfaceDetail) -> Color {
    // Basic continental/ocean model
    // In real application we'd use texture data or computed masks
    let lat = latitude.to_radians();
    let lon = longitude.to_radians();

    // Simplified model - in reality this would be based on a real land mask
    let lat_factor = lat.sin().abs().powf(2.0);

    // Earth-like hue with basic continent pattern
    if lat_factor < 0.2 {
        // Polar regions (ice)
        Color::Rgb {
            r: 200,
            g: 200,
            b: 220,
        }
    } else if lat_factor < 0.7 && (lon.sin().abs() > 0.3 || lat.cos().abs() > 0.3) {
        // Land areas - green
        let green = 180.0 - 100.0 * lat_factor;
        Color::Rgb {
            r: 60,
            g: green as u8,
            b: 80,
        }
    } else {
        // Ocean areas - blue
        let blue = 180.0 - 50.0 * lat_factor;
        Color::Rgb {
            r: 50,
            g: 80,
            b: blue as u8,
        }
    }
}

fn jupiter_albedo(latitude: f64, longitude: f64, detail: SurfaceDetail) -> Color {
    // Simplified Jupiter bands
    let lat = latitude.abs().to_radians();

    // Simple band model
    let band_index = (lat / (std::f64::consts::PI / 6.0)).floor() as i32;

    // Create alternating bands for Jupiter
    match band_index % 3 {
        0 => {
            // Light band - cream/brown
            Color::Rgb {
                r: 220,
                g: 180,
                b: 140,
            }
        }
        1 => {
            // Dark band - brown
            Color::Rgb {
                r: 180,
                g: 120,
                b: 80,
            }
        }
        _ => {
            // Equatorial zone - lighter
            Color::Rgb {
                r: 200,
                g: 160,
                b: 120,
            }
        }
    }
}
