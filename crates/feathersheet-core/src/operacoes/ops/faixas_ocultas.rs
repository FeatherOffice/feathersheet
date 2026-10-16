//! Faixas de linhas ocultas.



/// Linhas escondidas (altura 0) como faixas `[início, fim]`, em ordem e sem
/// sobreposição. Um filtro que esconde 1,7 milhão de linhas vira algumas
/// faixas — mandar linha por linha pela ponte seria o arquivo inteiro.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FaixasOcultas(pub(super) Vec<(u32, u32)>);

impl FaixasOcultas {
    /// Aceita em qualquer ordem, e junta as que se tocam.
    pub fn new(mut faixas: Vec<(u32, u32)>) -> Self {
        faixas.retain(|(a, b)| a <= b);
        faixas.sort_unstable();
        let mut juntas: Vec<(u32, u32)> = Vec::with_capacity(faixas.len());
        for (a, b) in faixas {
            match juntas.last_mut() {
                Some(ultima) if a <= ultima.1.saturating_add(1) => ultima.1 = ultima.1.max(b),
                _ => juntas.push((a, b)),
            }
        }
        FaixasOcultas(juntas)
    }

    pub fn contem(&self, linha: u32) -> bool {
        match self.0.binary_search_by(|&(a, _)| a.cmp(&linha)) {
            Ok(_) => true,
            Err(0) => false,
            Err(i) => linha <= self.0[i - 1].1,
        }
    }
}

