//! Alt+= (AutoSoma).

use crate::celulas::refs::cell_name;
use crate::celulas::values::Value;

/// Alt+= (a AutoSoma do Excel): a fórmula que soma o bloco de números logo
/// acima da célula, ou, sem nada acima, o da esquerda. `None` quando não há
/// número encostado. Era o `autoSoma` do `app.ts` (23/09/2026), que lia os
/// valores pela ponte célula a célula.
pub fn formula_da_autosoma(row: u32, col: u32, valor: impl Fn(u32, u32) -> Value) -> Option<String> {
    let numero = |r: u32, c: u32| matches!(valor(r, c), Value::Number(_));
    let mut r0 = row;
    while r0 > 0 && numero(r0 - 1, col) {
        r0 -= 1;
    }
    if r0 < row {
        return Some(format!("=SOMA({}:{})", cell_name(r0, col), cell_name(row - 1, col)));
    }
    let mut c0 = col;
    while c0 > 0 && numero(row, c0 - 1) {
        c0 -= 1;
    }
    if c0 < col {
        return Some(format!("=SOMA({}:{})", cell_name(row, c0), cell_name(row, col - 1)));
    }
    None
}

