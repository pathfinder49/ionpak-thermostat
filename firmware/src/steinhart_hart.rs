use libm::F32Ext;

/// Steinhart-Hart equation parameters
#[derive(Clone, Debug)]
pub struct Parameters {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    /// Parallel resistance
    ///
    /// Not truly part of the equation but required to calculate
    /// resistance from voltage.
    pub parallel_r: f32,
}

impl Parameters {
    /// input: Voltage
    ///
    /// Result unit: Kelvin
    pub fn get_temperature(&self, input: f32) -> f32 {
        let r = self.parallel_r * input;
        let ln_r = r.ln();
        let inv_temp = self.a +
            self.b * ln_r +
            self.c * ln_r * ln_r * ln_r;
        1.0 / inv_temp
    }
}
