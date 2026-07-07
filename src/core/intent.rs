// Intent Encoding Baseline - Locked by User
// All dimensions set to 0.00 (Neutral Baseline)
// Rule: Received -> Silence. No response generated.

pub struct IntentEncoding {
    pub emotion: f32,
    pub cognitive: f32,
    pub behavior: f32,
    pub relation: f32,
    pub memory: f32,
    pub creative: f32,
    pub domain: f32,
    pub urgency: f32,
}

impl IntentEncoding {
    pub fn new() -> Self {
        Self {
            emotion: 0.00,
            cognitive: 0.00,
            behavior: 0.00,
            relation: 0.00,
            memory: 0.00,
            creative: 0.00,
            domain: 0.00,
            urgency: 0.00,
        }
    }
}