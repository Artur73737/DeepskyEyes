//! Selettore ottica: wide / ultrawide / telephoto / front.
//! Mostra solo le ottiche annunciate dal discovery (niente voci fittizie).
pub struct LensSelectorView {
    pub available: Vec<String>,
    pub selected: Option<String>,
}
