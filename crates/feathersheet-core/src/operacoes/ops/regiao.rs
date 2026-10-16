//! Região atual (Ctrl+*).

use crate::planilha::sheet::Sheet;

/// O bloco de dados em volta da seleção — a "Região atual" do Excel
/// (`regiaoAtual` do `ops.ts`, com os mesmos casos de teste).
///
/// Cresce para os quatro lados enquanto a linha ou a coluna vizinha tiver
/// alguma célula preenchida, contando a diagonal; para em linha e coluna
/// vazias. A seleção de coluna inteira é cortada na área usada antes.
pub fn regiao_atual(sheet: &Sheet, sel: crate::celulas::refs::Rect) -> crate::celulas::refs::Rect {
    let sel = sel.normalized();
    if sheet.is_empty() {
        return sel;
    }
    let usado = sheet.used_rect();
    let (max_r, max_c) = (usado.r1, usado.c1);
    let mut r0 = sel.r0.min(max_r);
    let mut c0 = sel.c0.min(max_c);
    let mut r1 = sel.r1.min(max_r).max(r0);
    let mut c1 = sel.c1.min(max_c).max(c0);
    // `has_content` não monta o texto da célula: com 1 milhão de linhas ×
    // 85 colunas, o `get_raw` antigo montava 100 milhões de textos.
    let tem = |r: u32, c: u32| sheet.has_content(r, c);
    let coluna_tem = |c: u32, ra: u32, rb: u32| (ra..=rb.min(max_r)).any(|r| tem(r, c));
    // A linha `r` tem algo entre as colunas `ca..=cb`? Devolve qual coluna.
    let coluna_com = |r: u32, ca: u32, cb: u32| (ca..=cb.min(max_c)).find(|&c| tem(r, c));

    let mut mudou = true;
    while mudou {
        mudou = false;
        // Para baixo e para cima como o Ctrl+Shift+seta: achada uma coluna
        // com dado na linha vizinha, pula o bloco inteiro dela de uma vez,
        // em vez de crescer linha por linha.
        while r1 < max_r {
            let Some(c) = coluna_com(r1 + 1, c0.saturating_sub(1), c1 + 1) else { break };
            r1 += 1;
            while r1 < max_r && tem(r1 + 1, c) { r1 += 1; }
            mudou = true;
        }
        while r0 > 0 {
            let Some(c) = coluna_com(r0 - 1, c0.saturating_sub(1), c1 + 1) else { break };
            r0 -= 1;
            while r0 > 0 && tem(r0 - 1, c) { r0 -= 1; }
            mudou = true;
        }
        while c1 < max_c && coluna_tem(c1 + 1, r0.saturating_sub(1), r1 + 1) {
            c1 += 1;
            mudou = true;
        }
        while c0 > 0 && coluna_tem(c0 - 1, r0.saturating_sub(1), r1 + 1) {
            c0 -= 1;
            mudou = true;
        }
    }
    crate::celulas::refs::Rect::new(r0, c0, r1, c1)
}

