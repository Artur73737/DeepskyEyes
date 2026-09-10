//! Tipi sessione: LIGHT/DARK/FLAT/BIAS/TEST (+FOCUS futuri).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    Light,
    Dark,
    Flat,
    Bias,
    Test,
}
