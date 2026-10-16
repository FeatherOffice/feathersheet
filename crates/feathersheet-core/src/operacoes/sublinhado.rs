//! Sublinhado simples e duplo (Página Inicial > Fonte > Sublinhado ▾).
//!
//! O estilo guarda `underline: true` nos dois casos e, no duplo, também
//! `doubleUnderline: true`. Assim quem só conhece o sublinhado simples
//! (impressão em PDF, regras antigas) continua mostrando um traço, e o
//! `.xlsx` grava `<u/>` ou `<u val="double"/>` conforme o caso.
use crate::celulas::estilo_parcial::{Campo, EstiloParcial};
use crate::celulas::refs::Rect;
use crate::planilha::sheet::Sheet;

/// Chave do estilo que marca o sublinhado duplo.
pub const CHAVE_DUPLO: &str = "doubleUnderline";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sublinhado {
    Nenhum,
    Simples,
    Duplo,
}

/// O sublinhado de um estilo já mesclado.
pub fn sublinhado_do_estilo(e: &EstiloParcial) -> Sublinhado {
    let liga = |k: &str| matches!(e.get(k), Some(Campo::Logico(true)));
    if liga(CHAVE_DUPLO) {
        Sublinhado::Duplo
    } else if liga("underline") {
        Sublinhado::Simples
    } else {
        Sublinhado::Nenhum
    }
}

/// O sublinhado que vale na célula (aba, coluna, linha e célula).
pub fn sublinhado_de(sheet: &Sheet, linha: u32, coluna: u32) -> Sublinhado {
    sheet.estilo_mesclado(linha, coluna).as_ref()
        .map(sublinhado_do_estilo).unwrap_or(Sublinhado::Nenhum)
}

/// O pedaço de estilo que deixa a célula com o sublinhado `tipo`.
pub fn patch_sublinhado(tipo: Sublinhado) -> EstiloParcial {
    let mut p = EstiloParcial::new();
    let (simples, duplo) = match tipo {
        Sublinhado::Nenhum => (Campo::Nulo, Campo::Nulo),
        Sublinhado::Simples => (Campo::Logico(true), Campo::Nulo),
        Sublinhado::Duplo => (Campo::Logico(true), Campo::Logico(true)),
    };
    p.insert("underline".into(), simples);
    p.insert(CHAVE_DUPLO.into(), duplo);
    p
}

/// Como o botão do Excel: se a célula ativa já tem esse sublinhado, tira;
/// senão põe `tipo` em todas as áreas. Um Ctrl+Z desfaz tudo. Devolve o
/// sublinhado aplicado.
pub fn alternar_sublinhado(sheet: &mut Sheet, areas: &[Rect], ativa: (u32, u32), tipo: Sublinhado) -> Sublinhado {
    let novo = if sublinhado_de(sheet, ativa.0, ativa.1) == tipo { Sublinhado::Nenhum } else { tipo };
    let patch = patch_sublinhado(novo);
    sheet.abrir_transacao();
    for area in areas {
        sheet.set_style(area, &patch);
    }
    sheet.fechar_transacao();
    novo
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn duplo_e_simples_se_excluem_e_desfazem_de_uma_vez() {
        let mut s = Sheet::new("T");
        let a = [Rect::new(0, 0, 1, 1)];
        assert_eq!(alternar_sublinhado(&mut s, &a, (0, 0), Sublinhado::Duplo), Sublinhado::Duplo);
        assert_eq!(sublinhado_de(&s, 1, 1), Sublinhado::Duplo);
        // O duplo também é sublinhado para quem só conhece o simples.
        assert!(s.estilo_resolvido(0, 0).unwrap().underline);
        assert_eq!(alternar_sublinhado(&mut s, &a, (0, 0), Sublinhado::Simples), Sublinhado::Simples);
        assert_eq!(sublinhado_de(&s, 0, 1), Sublinhado::Simples);
        assert!(!s.estilo_mesclado(0, 1).unwrap().contains_key(CHAVE_DUPLO));
        // Repetir o mesmo tira.
        assert_eq!(alternar_sublinhado(&mut s, &a, (0, 0), Sublinhado::Simples), Sublinhado::Nenhum);
        assert_eq!(sublinhado_de(&s, 0, 0), Sublinhado::Nenhum);
        assert!(s.undo());
        assert_eq!(sublinhado_de(&s, 1, 0), Sublinhado::Simples);
        assert!(s.undo());
        assert_eq!(sublinhado_de(&s, 1, 0), Sublinhado::Duplo);
        assert!(s.undo());
        assert_eq!(sublinhado_de(&s, 1, 0), Sublinhado::Nenhum);
    }
}
