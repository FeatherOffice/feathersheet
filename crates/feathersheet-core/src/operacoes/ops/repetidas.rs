//! Remover linhas repetidas.

use std::collections::HashSet;
use crate::celulas::refs::Rect;
use crate::celulas::values::{to_text, Value};
use crate::planilha::sheet::Sheet;
use super::comum::{escrever, foto_da_linha};

/// O que o `removeDuplicates` decidiu: as linhas que ficam, na ordem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanoDeRepetidas {
    pub first: u32,
    pub max_r: u32,
    pub ficam: Vec<u32>,
    pub removidas: usize,
}

/// A metade que lê do `removeDuplicates`. `colunas` são as que decidem a
/// repetição (índices absolutos; todas as da área quando `None` ou vazio) —
/// a linha inteira da área sai junta (22/09/2026). Compara-se o valor
/// mostrado, não o texto da fórmula.
pub fn planejar_repetidas(
    area: Rect,
    has_header: bool,
    colunas: Option<&[u32]>,
    max_row: u32,
    valor: impl Fn(u32, u32) -> Value,
) -> Option<PlanoDeRepetidas> {
    let r = area.normalized();
    let first = r.r0 + u32::from(has_header);
    let max_r = r.r1.min(max_row);
    if first > max_r {
        return None;
    }
    let chaves: Vec<u32> = match colunas {
        Some(cs) if !cs.is_empty() => cs.iter().copied().filter(|c| (r.c0..=r.c1).contains(c)).collect(),
        _ => (r.c0..=r.c1).collect(),
    };
    if chaves.is_empty() {
        return None;
    }
    let mut vistos: HashSet<String> = HashSet::new();
    let mut ficam = Vec::new();
    let mut removidas = 0;
    let mut chave = String::new();
    for row in first..=max_r {
        chave.clear();
        for (i, &c) in chaves.iter().enumerate() {
            if i > 0 {
                chave.push('\u{0}');
            }
            chave.push_str(&to_text(&valor(row, c)));
        }
        if vistos.contains(&chave) {
            removidas += 1;
        } else {
            vistos.insert(chave.clone());
            ficam.push(row);
        }
    }
    (removidas > 0).then_some(PlanoDeRepetidas { first, max_r, ficam, removidas })
}

/// A metade que escreve: compacta no lugar, copiando para cima cada linha
/// que fica. O texto vai como estava, sem ajustar referência — igual ao
/// `write` do TypeScript (uma fórmula que sobe continua apontando para onde
/// apontava). Traduzido, não consertado.
pub fn aplicar_repetidas(sheet: &mut Sheet, c0: u32, c1: u32, plano: &PlanoDeRepetidas) {
    sheet.abrir_transacao();
    for (i, &origem) in plano.ficam.iter().enumerate() {
        let destino = plano.first + i as u32;
        if origem == destino {
            continue;
        }
        let foto = foto_da_linha(sheet, origem, c0, c1);
        sheet.clear(&Rect::new(destino, c0, destino, c1));
        for (c, (raw, estilo)) in (c0..=c1).zip(foto) {
            escrever(sheet, destino, c, &raw, estilo.as_ref());
        }
    }
    let ultima = plano.first + plano.ficam.len() as u32 - 1;
    if ultima < plano.max_r {
        sheet.clear(&Rect::new(ultima + 1, c0, plano.max_r, c1));
    }
    sheet.fechar_transacao();
}

/// `removeDuplicates`, lendo o `Sheet` sozinho. Devolve quantas saíram.
pub fn remove_duplicates(sheet: &mut Sheet, area: Rect, has_header: bool, colunas: Option<&[u32]>) -> usize {
    let max_row = sheet.max_row_ts();
    let plano = {
        let s = &*sheet;
        planejar_repetidas(area.clone(), has_header, colunas, max_row, |r, c| s.get_value(r, c))
    };
    let Some(p) = plano else { return 0 };
    let r = area.normalized();
    aplicar_repetidas(sheet, r.c0, r.c1, &p);
    p.removidas
}

