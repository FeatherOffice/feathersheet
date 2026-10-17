//! Métricas do motor para o Console do aplicativo.
//!
//! O motor não conhece a janela: anota aqui e a interface recolhe e mostra
//! no Console (F12). Nada vai para o terminal. O motor só escreve em
//! português; quem anota com [`anotar!`](crate::anotar) deixa junto o modelo
//! do `format!`, para a interface traduzir a linha para o idioma dela.

use std::sync::Mutex;

/// Acima disto as mais antigas saem: ninguém recolhendo não enche a RAM.
const LIMITE: usize = 500;

/// Uma linha do diário: o modelo em português (quando houver) e o texto pronto.
pub type Linha = (Option<&'static str>, String);

static FILA: Mutex<Vec<Linha>> = Mutex::new(Vec::new());

fn guardar(linha: Linha) {
    if let Ok(mut fila) = FILA.lock() {
        if fila.len() >= LIMITE {
            fila.remove(0);
        }
        fila.push(linha);
    }
}

/// Deixa uma linha para o Console, como `[abrir] aba 'Dados': …`.
pub fn anotar(mensagem: impl Into<String>) {
    guardar((None, mensagem.into()));
}

/// Como [`anotar`], guardando o modelo do `format!` (use [`anotar!`](crate::anotar)).
pub fn anotar_modelo(modelo: &'static str, texto: String) {
    guardar((Some(modelo), texto));
}

/// Tudo o que foi anotado desde a última vez, na ordem.
pub fn recolher() -> Vec<String> {
    recolher_com_modelo().into_iter().map(|(_, texto)| texto).collect()
}

/// Como [`recolher`], com o modelo de cada linha para traduzir.
pub fn recolher_com_modelo() -> Vec<Linha> {
    FILA.lock().map(|mut fila| std::mem::take(&mut *fila)).unwrap_or_default()
}

/// `anotar!("modelo {}", args…)`: `format!` + diário, com o modelo junto.
#[macro_export]
macro_rules! anotar {
    ($modelo:literal $($resto:tt)*) => {
        $crate::diario::anotar_modelo($modelo, format!($modelo $($resto)*))
    };
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn recolhe_na_ordem_e_esvazia() {
        anotar("[teste] um");
        crate::anotar!("[teste] {}", "dois");
        let linhas: Vec<_> = recolher_com_modelo().into_iter().filter(|(_, l)| l.starts_with("[teste]")).collect();
        assert_eq!(linhas, [(None, "[teste] um".to_owned()), (Some("[teste] {}"), "[teste] dois".to_owned())]);
        assert!(!recolher().iter().any(|l| l.starts_with("[teste]")));
    }
}
