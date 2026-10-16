//! "Ir para especial" do Excel: acha as células de um tipo (em branco,
//! constantes, fórmulas, diferenças) dentro de uma área e junta o resultado
//! em retângulos, para a interface selecionar várias áreas de uma vez.
use crate::celulas::{refs::Rect, values::Value};
use crate::planilha::sheet::Sheet;

/// Que tipos de valor contam em "Constantes" e "Fórmulas".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tipos {
    pub numeros: bool,
    pub texto: bool,
    pub logicos: bool,
    pub erros: bool,
}

impl Default for Tipos {
    fn default() -> Self { Self { numeros: true, texto: true, logicos: true, erros: true } }
}

impl Tipos {
    fn aceita(&self, v: &Value) -> bool {
        match v {
            Value::Number(_) => self.numeros,
            Value::Text(_) => self.texto,
            Value::Bool(_) => self.logicos,
            Value::Error(_) => self.erros,
            Value::Empty => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Especial {
    EmBranco,
    Constantes(Tipos),
    Formulas(Tipos),
    /// Células cujo valor difere do da coluna ativa, linha a linha.
    DiferencasNaLinha,
    /// Células cujo valor difere do da linha ativa, coluna a coluna.
    DiferencasNaColuna,
}

/// Células da `area` que atendem ao critério, em ordem de linha.
/// `valor` devolve o valor calculado (fórmulas incluídas). A área já deve
/// vir cortada na área usada: este laço visita cada célula dela.
pub fn localizar(sheet: &Sheet, area: &Rect, especial: Especial, ativa: (u32, u32),
    valor: impl Fn(u32, u32) -> Value) -> Vec<(u32, u32)> {
    let mut achadas = Vec::new();
    for r in area.r0..=area.r1 {
        for c in area.c0..=area.c1 {
            let bate = match especial {
                Especial::EmBranco => sheet.get_raw(r, c).is_empty(),
                Especial::Constantes(t) => {
                    let raw = sheet.get_raw(r, c);
                    !raw.is_empty() && !raw.starts_with('=') && t.aceita(&valor(r, c))
                }
                Especial::Formulas(t) =>
                    sheet.get_raw(r, c).starts_with('=') && t.aceita(&valor(r, c)),
                Especial::DiferencasNaLinha =>
                    c != ativa.1 && valor(r, c) != valor(r, ativa.1),
                Especial::DiferencasNaColuna =>
                    r != ativa.0 && valor(r, c) != valor(ativa.0, c),
            };
            if bate { achadas.push((r, c)); }
        }
    }
    achadas
}

/// Junta células (em ordem de linha) em retângulos: trechos seguidos de uma
/// linha viram faixas, e faixas iguais em linhas seguidas viram um bloco.
pub fn agrupar(celulas: &[(u32, u32)]) -> Vec<Rect> {
    // 1. Faixas horizontais: (linha, c0, c1).
    let mut faixas: Vec<(u32, u32, u32)> = Vec::new();
    for &(r, c) in celulas {
        match faixas.last_mut() {
            Some((lr, _, c1)) if *lr == r && *c1 + 1 == c => *c1 = c,
            _ => faixas.push((r, c, c)),
        }
    }
    // 2. Empilha faixas com as mesmas colunas em linhas seguidas.
    let mut abertos: Vec<Rect> = Vec::new();
    let mut prontos: Vec<Rect> = Vec::new();
    let mut linha_atual = None;
    let mut desta_linha: Vec<Rect> = Vec::new();
    for (r, c0, c1) in faixas {
        if linha_atual != Some(r) {
            // Blocos que não continuaram na linha anterior estão fechados.
            let continua = linha_atual.is_some_and(|l: u32| l + 1 == r);
            for bloco in abertos.drain(..) {
                if continua { desta_linha.push(bloco); } else { prontos.push(bloco); }
            }
            abertos = std::mem::take(&mut desta_linha);
            linha_atual = Some(r);
        }
        if let Some(i) = abertos.iter().position(|b| b.c0 == c0 && b.c1 == c1 && b.r1 + 1 == r) {
            let mut bloco = abertos.remove(i);
            bloco.r1 = r;
            desta_linha.push(bloco);
        } else {
            desta_linha.push(Rect::new(r, c0, r, c1));
        }
    }
    prontos.extend(abertos);
    prontos.extend(desta_linha);
    prontos.sort_by_key(|b| (b.r0, b.c0));
    prontos
}

#[cfg(test)]
mod testes {
    use super::*;

    fn folha(celulas: &[((u32, u32), &str)]) -> Sheet {
        let mut s = Sheet::new("A");
        for ((r, c), t) in celulas { s.set_cell(*r, *c, t); }
        s
    }

    #[test]
    fn constantes_formulas_e_em_branco_respeitam_os_tipos() {
        let s = folha(&[((0, 0), "1"), ((0, 1), "abc"), ((1, 0), "=1+1"), ((1, 1), "=\"x\"")]);
        let area = Rect::new(0, 0, 2, 1);
        let v = |r, c| s.get_value(r, c);
        assert_eq!(localizar(&s, &area, Especial::Constantes(Tipos::default()), (0, 0), v),
            vec![(0, 0), (0, 1)]);
        let so_numeros = Tipos { texto: false, ..Tipos::default() };
        assert_eq!(localizar(&s, &area, Especial::Formulas(so_numeros), (0, 0), v), vec![(1, 0)]);
        assert_eq!(localizar(&s, &area, Especial::EmBranco, (0, 0), v), vec![(2, 0), (2, 1)]);
    }

    #[test]
    fn diferencas_comparam_com_a_coluna_ou_linha_ativa() {
        let s = folha(&[((0, 0), "1"), ((0, 1), "1"), ((0, 2), "2"), ((1, 0), "5"), ((1, 1), "6")]);
        let area = Rect::new(0, 0, 1, 2);
        let v = |r, c| s.get_value(r, c);
        assert_eq!(localizar(&s, &area, Especial::DiferencasNaLinha, (0, 0), v),
            vec![(0, 2), (1, 1), (1, 2)]);
        assert_eq!(localizar(&s, &area, Especial::DiferencasNaColuna, (0, 0), v),
            vec![(1, 0), (1, 1), (1, 2)]);
    }

    #[test]
    fn agrupar_junta_trechos_e_blocos() {
        // Bloco A1:B2, célula solta D1 e faixa A4:C4.
        let celulas = [(0, 0), (0, 1), (0, 3), (1, 0), (1, 1), (3, 0), (3, 1), (3, 2)];
        assert_eq!(agrupar(&celulas), vec![
            Rect::new(0, 0, 1, 1), Rect::new(0, 3, 0, 3), Rect::new(3, 0, 3, 2),
        ]);
        assert!(agrupar(&[]).is_empty());
    }
}
