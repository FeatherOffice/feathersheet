//! A árvore de um padrão da coluna empacotada, analisada uma vez só.
//!
//! A coluna empacotada guarda o padrão (`=E{l}*F{l}`), não a árvore: cada
//! cálculo expandia o texto e analisava de novo — ~2 µs por `SE` aninhado,
//! contra ~150 ns de copiar uma árvore pronta (medido em 13/10/2026). Aqui a
//! árvore é montada na primeira célula e, para as outras, copiada com as
//! referências relativas deslocadas.

use crate::celulas::refs::{MAX_COLS, MAX_ROWS};
use crate::formula::ast::Node;
use crate::formula::formula_pattern::expand_pattern;
use crate::formula::parser::parse_formula;

/// Quais coordenadas de cada referência andam com a célula, na ordem do
/// [`Node::walk`]: `[linha, coluna]` de um `Ref`, `[r0, c0, r1, c1]` de um
/// `Range`.
#[derive(Debug)]
enum Relativa {
    Ref([bool; 2]),
    Range([bool; 4]),
}

#[derive(Debug)]
pub struct ModeloPadrao {
    linha: u32,
    coluna: u32,
    arvore: Node,
    refs: Vec<Relativa>,
}

fn analisar(padrao: &str, row: u32, col: u32) -> Option<Node> {
    let texto = expand_pattern(padrao, row as i32, col as i32);
    parse_formula(texto.strip_prefix('=').unwrap_or(&texto)).ok()
}

/// As coordenadas de cada referência, na ordem do `walk`.
fn coordenadas(arvore: &Node) -> Vec<(bool, [u32; 4])> {
    let mut v = Vec::new();
    arvore.walk(&mut |n| match n {
        Node::Ref { row, col, .. } => v.push((false, [*row, *col, 0, 0])),
        Node::Range { r0, c0, r1, c1, .. } => v.push((true, [*r0, *c0, *r1, *c1])),
        _ => {}
    });
    v
}

impl ModeloPadrao {
    /// Monta pela célula (`row`, `col`). `None` quando a comparação com a
    /// vizinha não fecha (a fórmula muda de forma de uma linha para outra):
    /// quem chama analisa o texto, como antes.
    pub fn montar(padrao: &str, row: u32, col: u32) -> Option<Self> {
        let (r2, c2) = (row.checked_add(1).filter(|r| *r < MAX_ROWS)?, col.checked_add(1).filter(|c| *c < MAX_COLS)?);
        let arvore = analisar(padrao, row, col)?;
        let vizinha = analisar(padrao, r2, c2)?;
        let (a, b) = (coordenadas(&arvore), coordenadas(&vizinha));
        if a.len() != b.len() {
            return None;
        }
        let mut refs = Vec::with_capacity(a.len());
        for ((faixa_a, ca), (faixa_b, cb)) in a.iter().zip(&b) {
            if faixa_a != faixa_b {
                return None;
            }
            // Relativa anda exatamente 1 (a vizinha está uma linha e uma
            // coluna adiante); fixa não anda. Qualquer outra coisa: desiste.
            let mut rel = [false; 4];
            for i in 0..4 {
                match cb[i] as i64 - ca[i] as i64 {
                    0 => {}
                    1 => rel[i] = true,
                    _ => return None,
                }
            }
            // Linha nas posições 0 e 2, coluna em 1 e 3.
            refs.push(if *faixa_a { Relativa::Range(rel) } else { Relativa::Ref([rel[0], rel[1]]) });
        }
        // A árvore de comparação também tem de ter a mesma forma no resto.
        if !mesma_forma(&arvore, &vizinha) {
            return None;
        }
        Some(ModeloPadrao { linha: row, coluna: col, arvore, refs })
    }

    /// A árvore da célula (`row`, `col`) do mesmo padrão. `None` se alguma
    /// referência sair da planilha: quem chama analisa o texto, que dá o
    /// erro certo.
    pub fn arvore_em(&self, row: u32, col: u32) -> Option<Node> {
        let (dr, dc) = (row as i64 - self.linha as i64, col as i64 - self.coluna as i64);
        let mover = |v: &mut u32, rel: bool, d: i64, max: u32| -> bool {
            if !rel { return true; }
            let novo = *v as i64 + d;
            if novo < 0 || novo >= max as i64 { return false; }
            *v = novo as u32;
            true
        };
        let mut arvore = self.arvore.clone();
        let mut i = 0;
        let mut ok = true;
        arvore.walk_mut(&mut |n| {
            if !ok { return; }
            match (n, self.refs.get(i)) {
                (Node::Ref { row, col, .. }, Some(Relativa::Ref([rr, rc]))) => {
                    ok = mover(row, *rr, dr, MAX_ROWS) && mover(col, *rc, dc, MAX_COLS);
                    i += 1;
                }
                (Node::Range { r0, c0, r1, c1, .. }, Some(Relativa::Range([a, b, c, d]))) => {
                    ok = mover(r0, *a, dr, MAX_ROWS) && mover(c0, *b, dc, MAX_COLS)
                        && mover(r1, *c, dr, MAX_ROWS) && mover(c1, *d, dc, MAX_COLS);
                    i += 1;
                }
                (Node::Ref { .. } | Node::Range { .. }, _) => ok = false,
                _ => {}
            }
        });
        ok.then_some(arvore)
    }
}

/// Um intervalo **local** que o padrão lê (`AO:AO`, `A2:A10`, `$A$1:A5`),
/// como a coluna empacotada precisa para saber quem suja quem sem guardar
/// uma dependência por célula. Cada ponta `[r0, c0, r1, c1]` é
/// `(anda, valor)`: deslocamento em relação à célula da fórmula quando anda
/// com ela, posição fixa quando não anda (cifrão, ou `A:A`, que o padrão
/// textual guarda sem marcador).
#[derive(Debug, Clone, PartialEq)]
pub struct FaixaPadrao {
    pontas: [(bool, i64); 4],
}

impl FaixaPadrao {
    fn ponta(&self, i: usize, base: u32) -> Option<u32> {
        let (anda, v) = self.pontas[i];
        let p = if anda { base as i64 + v } else { v };
        u32::try_from(p).ok()
    }

    /// O retângulo `(r0, c0, r1, c1)` que a fórmula da célula (`row`, `col`)
    /// lê. `None` se sair da planilha.
    pub fn rect_em(&self, row: u32, col: u32) -> Option<(u32, u32, u32, u32)> {
        let (r0, c0, r1, c1) = (self.ponta(0, row)?, self.ponta(1, col)?, self.ponta(2, row)?, self.ponta(3, col)?);
        Some((r0.min(r1), c0.min(c1), r0.max(r1), c0.max(c1)))
    }

    /// As colunas que a fórmula da coluna `col` lê (não dependem da linha).
    pub fn colunas_em(&self, col: u32) -> Option<(u32, u32)> {
        let (c0, c1) = (self.ponta(1, col)?, self.ponta(3, col)?);
        Some((c0.min(c1), c0.max(c1)))
    }

    /// As linhas lidas não dependem da linha da fórmula (`A:A`, `$A$2:$A$9`):
    /// mexer dentro delas suja a coluna inteira do padrão.
    pub fn linhas_fixas(&self) -> bool {
        !self.pontas[0].0 && !self.pontas[2].0
    }

    /// Quando as duas pontas de linha andam com a fórmula: os deslocamentos
    /// (menor, maior). A fórmula da linha `R` lê `R + menor ..= R + maior`.
    pub fn janela_relativa(&self) -> Option<(i64, i64)> {
        let (a, b) = (self.pontas[0], self.pontas[2]);
        (a.0 && b.0).then(|| (a.1.min(b.1), a.1.max(b.1)))
    }

    /// Deslocamentos de linha das pontas que andam (para conferir se a
    /// fórmula cabe numa linha nova).
    pub fn deslocamentos_de_linha(&self) -> impl Iterator<Item = i64> + '_ {
        [self.pontas[0], self.pontas[2]].into_iter().filter(|p| p.0).map(|p| p.1)
    }

    /// As pontas como referências de célula, para quem só sabe consertar
    /// referência em edição estrutural.
    pub fn pontas_como_refs(&self) -> [crate::formula::formula_pattern::PatternRef; 2] {
        let r = |a: usize, b: usize| crate::formula::formula_pattern::PatternRef {
            row: self.pontas[a].1 as i32,
            col: self.pontas[b].1 as i32,
            row_lock: !self.pontas[a].0,
            col_lock: !self.pontas[b].0,
            sheet: None,
        };
        [r(0, 1), r(2, 3)]
    }
}

/// Onde o padrão é analisado para achar os intervalos: no meio da planilha,
/// para referências relativas para cima ou para a esquerda não saírem dela.
const BASE_LINHA: u32 = 50_000_000;
const BASE_COLUNA: u32 = 8_000;

/// Os intervalos locais do padrão, na ordem do `walk`. Compara a árvore em
/// duas células vizinhas na diagonal, como [`ModeloPadrao::montar`]: a
/// coordenada que anda 1 é relativa, a que fica é fixa. `None` quando o
/// padrão tem intervalo local e a comparação não fecha.
pub fn faixas_do_padrao(padrao: &str) -> Option<Vec<FaixaPadrao>> {
    fn locais(arvore: &Node) -> Vec<[u32; 4]> {
        let mut v = Vec::new();
        arvore.walk(&mut |n| {
            if let Node::Range { r0, c0, r1, c1, sheet: None } = n {
                v.push([*r0, *c0, *r1, *c1]);
            }
        });
        v
    }
    let Some(a) = analisar(padrao, BASE_LINHA, BASE_COLUNA) else {
        // Sem `:` não há intervalo; a análise pode falhar por outro motivo
        // (referência longe demais), que não é assunto daqui.
        return (!padrao.contains(':')).then(Vec::new);
    };
    let b = analisar(padrao, BASE_LINHA + 1, BASE_COLUNA + 1)?;
    let (a, b) = (locais(&a), locais(&b));
    if a.len() != b.len() {
        return None;
    }
    a.iter().zip(&b).map(|(pa, pb)| {
        let mut pontas = [(false, 0i64); 4];
        for i in 0..4 {
            let base = if i % 2 == 0 { BASE_LINHA } else { BASE_COLUNA } as i64;
            pontas[i] = match pb[i] as i64 - pa[i] as i64 {
                0 => (false, pa[i] as i64),
                1 => (true, pa[i] as i64 - base),
                _ => return None,
            };
        }
        Some(FaixaPadrao { pontas })
    }).collect()
}

/// As duas árvores são iguais fora as coordenadas das referências.
fn mesma_forma(a: &Node, b: &Node) -> bool {
    match (a, b) {
        (Node::Ref { sheet: s1, .. }, Node::Ref { sheet: s2, .. }) => s1 == s2,
        (Node::Range { sheet: s1, .. }, Node::Range { sheet: s2, .. }) => s1 == s2,
        (Node::Binary { op: o1, left: l1, right: r1 }, Node::Binary { op: o2, left: l2, right: r2 }) =>
            o1 == o2 && mesma_forma(l1, l2) && mesma_forma(r1, r2),
        (Node::Unary { op: o1, operand: x1 }, Node::Unary { op: o2, operand: x2 }) => o1 == o2 && mesma_forma(x1, x2),
        (Node::Call { name: n1, args: a1 }, Node::Call { name: n2, args: a2 }) =>
            n1 == n2 && a1.len() == a2.len() && a1.iter().zip(a2).all(|(x, y)| mesma_forma(x, y)),
        (Node::Array { rows: r1 }, Node::Array { rows: r2 }) =>
            r1.len() == r2.len() && r1.iter().zip(r2).all(|(x, y)|
                x.len() == y.len() && x.iter().zip(y).all(|(p, q)| mesma_forma(p, q))),
        (x, y) => x == y,
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::formula::formula_pattern::pattern_for;

    #[test]
    fn faixas_do_padrao_separa_fixa_de_relativa() {
        let p = pattern_for("=MÍNIMOSES(AO:AO;A:A;A2;C:C;C2;AP2:AP9;$B$1:B2)", 1, 46).unwrap();
        let f = faixas_do_padrao(&p).unwrap();
        assert_eq!(f.len(), 5, "AO:AO, A:A, C:C, AP2:AP9, $B$1:B2");
        // AO:AO na coluna AU, linha 2: fixa, coluna 40 inteira.
        assert!(f[0].linhas_fixas());
        assert_eq!(f[0].rect_em(1, 46), Some((0, 40, MAX_ROWS - 1, 40)));
        assert_eq!(f[0].rect_em(900, 46), Some((0, 40, MAX_ROWS - 1, 40)));
        // AP2:AP9 anda com a linha.
        assert!(!f[3].linhas_fixas());
        assert_eq!(f[3].rect_em(1, 46), Some((1, 41, 8, 41)));
        assert_eq!(f[3].rect_em(11, 46), Some((11, 41, 18, 41)));
        // $B$1:B2: uma ponta fixa, a outra anda.
        assert_eq!(f[4].rect_em(10, 46), Some((0, 1, 10, 1)));
        assert_eq!(f[4].colunas_em(46), Some((1, 1)));
        assert_eq!(faixas_do_padrao(&pattern_for("=A2*2", 1, 3).unwrap()), Some(vec![]));
    }

    fn confere(raw: &str, row: u32, col: u32, outra: (u32, u32)) {
        let padrao = pattern_for(raw, row as i32, col as i32).expect("padrão");
        let modelo = ModeloPadrao::montar(&padrao, row, col).expect("modelo");
        let esperado = analisar(&padrao, outra.0, outra.1);
        assert_eq!(modelo.arvore_em(outra.0, outra.1), esperado, "{raw} em {outra:?}");
    }

    #[test]
    fn arvore_deslocada_igual_a_analisada_do_texto() {
        confere("=E2*F2", 1, 6, (999_999, 6));
        confere("=K1+I2", 1, 10, (500, 10));
        confere("=A2*$C$1+B$1-$D2", 1, 4, (77, 9));
        confere("=SE(I2>2000;\"Alto\";SE(I2>800;\"Médio\";\"Baixo\"))", 1, 12, (42, 12));
    }
}
