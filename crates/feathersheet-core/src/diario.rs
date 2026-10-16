//! Métricas do motor para o Console do aplicativo.
//!
//! O motor não conhece a janela: anota aqui e a interface recolhe e mostra
//! no Console (F12). Nada vai para o terminal.

use std::sync::Mutex;

/// Acima disto as mais antigas saem: ninguém recolhendo não enche a RAM.
const LIMITE: usize = 500;

static FILA: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Deixa uma linha para o Console, como `[abrir] aba 'Dados': …`.
pub fn anotar(mensagem: impl Into<String>) {
    if let Ok(mut fila) = FILA.lock() {
        if fila.len() >= LIMITE {
            fila.remove(0);
        }
        fila.push(mensagem.into());
    }
}

/// Tudo o que foi anotado desde a última vez, na ordem.
pub fn recolher() -> Vec<String> {
    FILA.lock().map(|mut fila| std::mem::take(&mut *fila)).unwrap_or_default()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn recolhe_na_ordem_e_esvazia() {
        anotar("[teste] um");
        anotar("[teste] dois");
        let linhas: Vec<_> = recolher().into_iter().filter(|l| l.starts_with("[teste]")).collect();
        assert_eq!(linhas, ["[teste] um", "[teste] dois"]);
        assert!(!recolher().iter().any(|l| l.starts_with("[teste]")));
    }
}
