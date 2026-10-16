//! Detecção de cabeçalho de tabela.

use crate::celulas::refs::Rect;
use crate::celulas::values::Value;

/// Se a primeira linha da área parece cabeçalho: toda ela em texto, e alguma
/// coisa que não é texto logo embaixo. Tradução do `pareceCabecalho` do
/// `table.ts` — é o que decide o "Minha tabela tem cabeçalhos" do Ctrl+T.
pub fn parece_cabecalho(area: Rect, valor: impl Fn(u32, u32) -> Value) -> bool {
    let rect = area.normalized();
    if rect.r1 <= rect.r0 {
        return false;
    }
    let vazio = |v: &Value| matches!(v, Value::Empty) || matches!(v, Value::Text(t) if t.is_empty());
    let (mut textos_em_cima, mut nao_textos_em_baixo, mut colunas) = (0, 0, 0);
    for c in rect.c0..=rect.c1 {
        let cima = valor(rect.r0, c);
        if vazio(&cima) {
            continue;
        }
        colunas += 1;
        if matches!(cima, Value::Text(_)) {
            textos_em_cima += 1;
        }
        let baixo = valor(rect.r0 + 1, c);
        if !vazio(&baixo) && !matches!(baixo, Value::Text(_)) {
            nao_textos_em_baixo += 1;
        }
    }
    colunas > 0 && textos_em_cima == colunas && nao_textos_em_baixo > 0
}

