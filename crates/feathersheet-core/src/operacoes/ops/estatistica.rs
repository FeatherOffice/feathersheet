//! Estatísticas de coluna e da seleção (o que o rodapé mostra).

use std::collections::HashSet;
use crate::celulas::refs::Rect;
use crate::celulas::values::{to_text, Value};
use crate::planilha::sheet::Sheet;

#[derive(Debug, Clone, PartialEq)]
pub struct ColumnStats {
    pub count: usize,
    pub numbers: usize,
    pub empty: usize,
    pub distinct: usize,
    pub sum: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
}

/// `columnStats`: o "Estatísticas da coluna". Para na última linha usada
/// (`max_row`, o `maxRow` do TypeScript): selecionar a coluna inteira não
/// anda um milhão de linhas vazias.
pub fn column_stats_com(area: Rect, max_row: u32, valor: impl Fn(u32, u32) -> Value) -> ColumnStats {
    let rect = area.normalized();
    let max_r = rect.r1.min(max_row);
    let mut distinct: HashSet<String> = HashSet::new();
    let (mut count, mut numbers, mut empty) = (0usize, 0usize, 0usize);
    let (mut sum, mut min, mut max) = (0.0f64, f64::INFINITY, f64::NEG_INFINITY);
    if rect.r0 <= max_r {
        for r in rect.r0..=max_r {
            for c in rect.c0..=rect.c1 {
                let v = valor(r, c);
                if matches!(&v, Value::Empty) || matches!(&v, Value::Text(t) if t.is_empty()) {
                    empty += 1;
                    continue;
                }
                count += 1;
                distinct.insert(to_text(&v));
                if let Value::Number(n) = v {
                    numbers += 1;
                    sum += n;
                    if n < min {
                        min = n;
                    }
                    if n > max {
                        max = n;
                    }
                }
            }
        }
    }
    ColumnStats {
        count,
        numbers,
        empty,
        distinct: distinct.len(),
        sum,
        average: if numbers > 0 { sum / numbers as f64 } else { 0.0 },
        min: if numbers > 0 { min } else { 0.0 },
        max: if numbers > 0 { max } else { 0.0 },
    }
}

/// `columnStats` lendo o valor do próprio `Sheet`.
pub fn column_stats(sheet: &Sheet, area: Rect) -> ColumnStats {
    let max_row = if sheet.is_empty() { 0 } else { sheet.used_rect().r1 };
    column_stats_com(area, max_row, |r, c| sheet.get_value(r, c))
}

/// O que o rodapé mostra da seleção.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionStats {
    /// Células com conteúdo que não é erro.
    pub count: u64,
    /// Quantas delas são número — é por elas que se divide a média.
    pub numbers: u64,
    pub sum: f64,
    /// Soma dos quadrados dos números (quem quiser o desvio padrão).
    pub sum_sq: f64,
    pub min: f64,
    pub max: f64,
}

/// Soma, média, mínimo, máximo e contagem da seleção — o `somarEm` do
/// `statusbar.ts`, que é a referência.
///
/// Como lá: célula vazia e célula com erro não contam; texto e lógico contam
/// na contagem e não na soma; **linha escondida não conta** (altura 0, que é
/// como o filtro esconde — pedido do usuário em 22/09/2026, quando o rodapé
/// seguia somando as dois milhões de linhas fora do filtro).
///
/// Diferente de lá, é uma conta só: o TypeScript precisava fatiar por quadro
/// para não travar a tela, e isto roda fora dela.
pub fn selection_stats(sheet: &Sheet, area: crate::celulas::refs::Rect) -> SelectionStats {
    selection_stats_com(sheet, area, |r| sheet.row_heights.get(&r) != Some(&0), |r, c| sheet.get_value(r, c))
}

/// A mesma conta, com quem diz se a linha está à vista e quem lê o valor
/// vindos de fora. É o que a ponte usa: lá as alturas ainda moram no
/// TypeScript (chegam como [`FaixasOcultas`]) e o valor é lido pela pasta,
/// para a fórmula que lê outra aba funcionar.
pub fn selection_stats_com(
    sheet: &Sheet,
    area: crate::celulas::refs::Rect,
    mut visivel: impl FnMut(u32) -> bool,
    valor: impl Fn(u32, u32) -> Value,
) -> SelectionStats {
    let mut st =
        SelectionStats { count: 0, numbers: 0, sum: 0.0, sum_sq: 0.0, min: f64::INFINITY, max: f64::NEG_INFINITY };
    if sheet.is_empty() {
        return st;
    }
    let usado = sheet.used_rect();
    let (r0, r1) = (area.r0.max(usado.r0), area.r1.min(usado.r1));
    let (c0, c1) = (area.c0.max(usado.c0), area.c1.min(usado.c1));
    if r0 > r1 || c0 > c1 {
        return st;
    }
    for r in r0..=r1 {
        if !visivel(r) {
            continue;
        }
        for c in c0..=c1 {
            match valor(r, c) {
                Value::Empty | Value::Error(_) => {}
                Value::Number(n) => {
                    st.count += 1;
                    st.numbers += 1;
                    st.sum += n;
                    st.sum_sq += n * n;
                    st.min = st.min.min(n);
                    st.max = st.max.max(n);
                }
                _ => st.count += 1,
            }
        }
    }
    st
}

