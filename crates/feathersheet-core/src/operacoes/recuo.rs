//! Recuo (indent) do texto da célula: Aumentar/Diminuir recuo da faixa.
//!
//! O recuo é um campo do estilo da célula (`indent`, inteiro 0..=15, cada
//! nível ≈ 3 espaços, como no Excel), guardado no mesmo mapa do alinhamento;
//! por isso desfazer/refazer, o `.fshz` e o `.xlsx` (`<alignment indent>`)
//! já o tratam como qualquer outro campo de estilo.
use crate::celulas::estilo_parcial::{Campo, EstiloParcial};
use crate::celulas::refs::Rect;
use crate::planilha::sheet::Sheet;

/// Maior nível de recuo aceito (o do Excel).
pub const RECUO_MAX: u32 = 15;
/// Acima disto a seleção (coluna/linha inteira) usa o recuo da célula do
/// canto como base única, em vez de ler célula a célula.
const LIMITE_CELULAS: u64 = 10_000;

/// O nível de recuo que vale na célula (estilo de aba, coluna, linha e célula).
pub fn recuo_de(sheet: &Sheet, linha: u32, coluna: u32) -> u32 {
    match sheet.estilo_mesclado(linha, coluna).as_ref().and_then(|e| e.get("indent")) {
        Some(Campo::Numero(n)) if *n > 0.0 => (n.round() as u32).min(RECUO_MAX),
        _ => 0,
    }
}

fn patch_recuo(nivel: u32, alinhar_esquerda: bool) -> EstiloParcial {
    let mut p = EstiloParcial::new();
    p.insert("indent".into(), if nivel == 0 { Campo::Nulo } else { Campo::Numero(nivel as f64) });
    if alinhar_esquerda {
        // Como o Excel: recuo sobre o alinhamento Geral passa a ser à esquerda.
        p.insert("align".into(), Campo::Texto("left".into()));
    }
    p
}

fn alinhamento_automatico(sheet: &Sheet, linha: u32, coluna: u32) -> bool {
    !matches!(sheet.estilo_mesclado(linha, coluna).as_ref().and_then(|e| e.get("align")),
        Some(Campo::Texto(t)) if t == "left" || t == "right" || t == "center" || t == "justify")
}

/// Soma `delta` níveis ao recuo de cada célula da área (limitado a 0..=15),
/// numa só transação (um Ctrl+Z desfaz tudo). Devolve se algo mudou.
pub fn alterar_recuo(sheet: &mut Sheet, area: &Rect, delta: i32) -> bool {
    let r = area.normalized();
    let total = (r.r1 - r.r0 + 1) as u64 * (r.c1 - r.c0 + 1) as u64;
    let novo = |atual: u32| (atual as i64 + delta as i64).clamp(0, RECUO_MAX as i64) as u32;
    let mut mudou = false;
    sheet.abrir_transacao();
    if total > LIMITE_CELULAS {
        let atual = recuo_de(sheet, r.r0, r.c0);
        let n = novo(atual);
        if n != atual {
            let esq = delta > 0 && alinhamento_automatico(sheet, r.r0, r.c0);
            sheet.set_style(&r, &patch_recuo(n, esq));
            mudou = true;
        }
    } else {
        for l in r.r0..=r.r1 {
            for c in r.c0..=r.c1 {
                let atual = recuo_de(sheet, l, c);
                let n = novo(atual);
                if n == atual { continue; }
                let esq = delta > 0 && alinhamento_automatico(sheet, l, c);
                sheet.set_style(&Rect::cell(l, c), &patch_recuo(n, esq));
                mudou = true;
            }
        }
    }
    sheet.fechar_transacao();
    mudou
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn aumenta_diminui_e_respeita_limites() {
        let mut s = Sheet::new("T");
        let a = Rect::cell(0, 0);
        assert!(!alterar_recuo(&mut s, &a, -1), "já está em 0");
        assert!(alterar_recuo(&mut s, &a, 1));
        assert_eq!(recuo_de(&s, 0, 0), 1);
        // Alinhamento Geral vira à esquerda, como no Excel.
        assert_eq!(s.estilo_mesclado(0, 0).unwrap().get("align"), Some(&Campo::Texto("left".into())));
        for _ in 0..30 { alterar_recuo(&mut s, &a, 1); }
        assert_eq!(recuo_de(&s, 0, 0), RECUO_MAX);
        for _ in 0..30 { alterar_recuo(&mut s, &a, -1); }
        assert_eq!(recuo_de(&s, 0, 0), 0);
        assert!(!s.estilo_mesclado(0, 0).unwrap().contains_key("indent"));
    }

    #[test]
    fn cada_celula_soma_o_seu_proprio_nivel_e_desfaz_de_uma_vez() {
        let mut s = Sheet::new("T");
        alterar_recuo(&mut s, &Rect::cell(0, 0), 3);
        alterar_recuo(&mut s, &Rect::new(0, 0, 1, 0), 1);
        assert_eq!(recuo_de(&s, 0, 0), 4);
        assert_eq!(recuo_de(&s, 1, 0), 1);
        assert!(s.undo());
        assert_eq!(recuo_de(&s, 0, 0), 3);
        assert_eq!(recuo_de(&s, 1, 0), 0);
        assert!(s.redo());
        assert_eq!(recuo_de(&s, 0, 0), 4);
        assert_eq!(recuo_de(&s, 1, 0), 1);
    }

    #[test]
    fn alinhamento_a_direita_e_preservado() {
        let mut s = Sheet::new("T");
        let mut p = EstiloParcial::new();
        p.insert("align".into(), Campo::Texto("right".into()));
        s.set_style(&Rect::cell(0, 0), &p);
        alterar_recuo(&mut s, &Rect::cell(0, 0), 2);
        assert_eq!(s.estilo_mesclado(0, 0).unwrap().get("align"), Some(&Campo::Texto("right".into())));
        assert_eq!(recuo_de(&s, 0, 0), 2);
    }
}
