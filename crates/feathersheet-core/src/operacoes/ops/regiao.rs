//! Região atual (Ctrl+*).

use crate::planilha::sheet::Sheet;
use std::collections::HashMap;

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
    // Cada coluna vira suas sequências de linhas cheias, lidas das máscaras
    // de presença bloco a bloco. Antes era `has_content` célula a célula: em
    // 13 milhões de linhas o Ctrl+T deixava a janela "não respondendo"
    // (17/10/2026).
    let mut colunas = Colunas { sheet, ate: max_r, lidas: HashMap::new() };

    let mut mudou = true;
    while mudou {
        mudou = false;
        // Para baixo e para cima como o Ctrl+Shift+seta: achada uma coluna
        // com dado na linha vizinha, pula a sequência inteira dela de uma vez.
        while r1 < max_r {
            let Some((_, fim)) = colunas.na_linha(r1 + 1, c0.saturating_sub(1), (c1 + 1).min(max_c)) else { break };
            r1 = fim.min(max_r);
            mudou = true;
        }
        while r0 > 0 {
            let Some((inicio, _)) = colunas.na_linha(r0 - 1, c0.saturating_sub(1), (c1 + 1).min(max_c)) else { break };
            r0 = inicio;
            mudou = true;
        }
        while c1 < max_c && colunas.tem_na_faixa(c1 + 1, r0.saturating_sub(1), r1 + 1) {
            c1 += 1;
            mudou = true;
        }
        while c0 > 0 && colunas.tem_na_faixa(c0 - 1, r0.saturating_sub(1), r1 + 1) {
            c0 -= 1;
            mudou = true;
        }
    }
    crate::celulas::refs::Rect::new(r0, c0, r1, c1)
}

/// As sequências `(primeira, última)` de linhas cheias de cada coluna
/// ([`Sheet::sequencias_com_conteudo`]), lidas uma vez só por coluna.
pub(crate) struct Colunas<'a> {
    sheet: &'a Sheet,
    ate: u32,
    lidas: HashMap<u32, Vec<(u32, u32)>>,
}

impl<'a> Colunas<'a> {
    pub(crate) fn novas(sheet: &'a Sheet, ate: u32) -> Self {
        Self { sheet, ate, lidas: HashMap::new() }
    }

    fn de(&mut self, c: u32) -> &[(u32, u32)] {
        let (sheet, ate) = (self.sheet, self.ate);
        self.lidas.entry(c).or_insert_with(|| sheet.sequencias_com_conteudo(c, ate))
    }

    /// A primeira sequência que termina em `r` ou depois.
    fn a_partir(&mut self, c: u32, r: u32) -> Option<(u32, u32)> {
        let s = self.de(c);
        s.get(s.partition_point(|&(_, fim)| fim < r)).copied()
    }

    /// Até onde vão, juntas, as linhas cheias a partir de `r` em alguma das
    /// colunas `ca..=cb` (`r` incluída; `None` se a linha `r` está vazia
    /// nelas). Para baixo com `desce`, para cima sem.
    pub(crate) fn bloco_de_linhas(&mut self, r: u32, ca: u32, cb: u32, desce: bool) -> Option<u32> {
        let mut ponta = r;
        let mut achou = false;
        loop {
            let alvo = if !achou { Some(ponta) } else if desce { ponta.checked_add(1) } else { ponta.checked_sub(1) };
            let Some(alvo) = alvo else { return Some(ponta) };
            let mut nova: Option<u32> = None;
            for c in ca..=cb {
                if let Some((inicio, fim)) = self.a_partir(c, alvo).filter(|&(inicio, _)| inicio <= alvo) {
                    let p = if desce { fim } else { inicio };
                    nova = Some(nova.map_or(p, |n| if desce { n.max(p) } else { n.min(p) }));
                }
            }
            match nova {
                Some(p) => { ponta = p; achou = true; }
                None => return achou.then_some(ponta),
            }
        }
    }

    /// A sequência que contém a linha `r` na primeira coluna de `ca..=cb`
    /// que tiver algo nela.
    fn na_linha(&mut self, r: u32, ca: u32, cb: u32) -> Option<(u32, u32)> {
        (ca..=cb).find_map(|c| self.a_partir(c, r).filter(|&(inicio, _)| inicio <= r))
    }

    /// A coluna tem algo entre as linhas `ra..=rb`?
    pub(crate) fn tem_na_faixa(&mut self, c: u32, ra: u32, rb: u32) -> bool {
        let rb = rb.min(self.ate);
        self.a_partir(c, ra).is_some_and(|(inicio, _)| inicio <= rb)
    }
}
