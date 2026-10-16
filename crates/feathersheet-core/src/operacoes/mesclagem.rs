//! Geometria de mesclagem compartilhada pela GUI e pelas macros.
//! Sem JSON, Iced, JS ou alocação de células: opera apenas em Vec<Rect>.
use crate::celulas::refs::{Rect, MAX_COLS, MAX_ROWS};

pub fn cruzam(a: &Rect, b: &Rect) -> bool {
    a.r0 <= b.r1 && b.r0 <= a.r1 && a.c0 <= b.c1 && b.c0 <= a.c1
}

pub fn valida(r: &Rect) -> bool {
    r.r0 <= r.r1 && r.c0 <= r.c1
        && r.r1 < MAX_ROWS && r.c1 < MAX_COLS
        && (r.r1 > r.r0 || r.c1 > r.c0)
}

/// A última mesclagem substitui áreas conflitantes, como a leitura
/// do metadado mc no FeatherSheet legado.
pub fn normalizar(registros: Vec<Rect>) -> Vec<Rect> {
    let mut saida = Vec::with_capacity(registros.len());
    for registro in registros {
        let n = registro.normalized();
        if valida(&n) {
            saida.retain(|anterior| !cruzam(anterior, &n));
            saida.push(n);
        }
    }
    saida
}

pub fn em(regioes: &[Rect], row: u32, col: u32) -> Option<&Rect> {
    regioes.iter().find(|r| r.contains(row, col))
}

/// Amplia a seleção para incluir todas as mesclagens que ela toca.
pub fn expandir(regioes: &[Rect], mut area: Rect) -> Rect {
    loop {
        let mut mudou = false;
        for r in regioes {
            if !cruzam(r, &area) { continue; }
            let proxima = Rect::new(
                area.r0.min(r.r0), area.c0.min(r.c0),
                area.r1.max(r.r1), area.c1.max(r.c1),
            );
            if proxima != area { area = proxima; mudou = true; }
        }
        if !mudou { return area; }
    }
}

pub fn mesclar(regioes: &[Rect], area: Rect) -> Option<Vec<Rect>> {
    let area = expandir(regioes, area.normalized());
    if !valida(&area) { return None; }
    let mut novas: Vec<_> = regioes.iter()
        .filter(|r| !cruzam(r, &area)).cloned().collect();
    novas.push(area);
    (novas != regioes).then_some(novas)
}

pub fn desmesclar(regioes: &[Rect], area: Rect) -> Option<Vec<Rect>> {
    let novas: Vec<_> = regioes.iter()
        .filter(|r| !cruzam(r, &area)).cloned().collect();
    (novas.len() != regioes.len()).then_some(novas)
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn mesclagem_cruzada_expande_e_unmerge_parcial_remove() {
        let inicial = mesclar(&[],Rect::new(0,0,1,1)).unwrap();
        assert!(mesclar(&inicial,Rect::new(0,0,1,1)).is_none());
        let expandidas = mesclar(&inicial,Rect::new(1,1,2,3)).unwrap();
        assert_eq!(expandidas,vec![Rect::new(0,0,2,3)]);
        assert!(em(&expandidas,2,2).is_some());
        let vazias=desmesclar(&expandidas,Rect::cell(1,1)).unwrap();
        assert!(vazias.is_empty());
    }
    #[test]
    fn normalizacao_descarta_unitarias_e_sobreposicao_antiga() {
        let rs=normalizar(vec![
            Rect::cell(0,0),
            Rect::new(1,1,2,2),
            Rect::new(2,2,3,4),
        ]);
        assert_eq!(rs,vec![Rect::new(2,2,3,4)]);
        assert!(!valida(&Rect::cell(0,0)));
    }
}
