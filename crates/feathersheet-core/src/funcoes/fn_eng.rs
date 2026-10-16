//! Funções de engenharia em Rust: conversões entre binário, octal, decimal e hexadecimal (BINADEC … OCTAHEX),
//! operações de bits (BITAND, BITOR, BITXOR, DESLOCESQBIT, DESLOCDIRBIT) e comparações (DELTA, DEGRAU).

use crate::celulas::values::{to_text, Computed, Value};
use crate::formula::ast::Node;
use crate::formula::evaluator::Evaluator;
use crate::funcoes::fn_helpers::{e_num, e_valor, numero, por_elemento, texto, Entradas, Res};

/// Uma base de numeração: raiz e quantidade de bits dos 10 dígitos do complemento de dois.
#[derive(Clone, Copy)]
struct Base {
    raiz: u32,
    bits: u32,
}

const BIN: Base = Base { raiz: 2, bits: 10 };
const OCT: Base = Base { raiz: 8, bits: 30 };
const HEX: Base = Base { raiz: 16, bits: 40 };

/// Lê o argumento 0 como número escrito na base (até 10 dígitos; com 10 dígitos
/// o bit mais alto é o sinal, em complemento de dois).
fn ler(e: &Entradas<'_>, base: Base) -> Res<i64> {
    let v = e.valor(0);
    let t = match v {
        Value::Error(er) => return Err(er.clone()),
        Value::Bool(_) => return Err(e_valor()),
        Value::Number(n) if *n < 0.0 => return Err(e_num()),
        outro => to_text(outro),
    };
    if t.chars().count() > 10 {
        return Err(e_num());
    }
    let mut n: i64 = 0;
    for c in t.chars() {
        let d = c.to_digit(base.raiz).ok_or_else(e_num)?;
        n = n * base.raiz as i64 + d as i64;
    }
    if t.chars().count() == 10 && n >= 1i64 << (base.bits - 1) {
        n -= 1i64 << base.bits;
    }
    Ok(n)
}

/// Escreve `n` na base de destino; negativo vira complemento de dois de 10
/// dígitos (e ignora as casas). Fora do intervalo da base é `#NÚM!`.
fn escrever(e: &Entradas<'_>, n: i64, base: Base) -> Res<Value> {
    let limite = 1i64 << (base.bits - 1);
    if n < -limite || n >= limite {
        return Err(e_num());
    }
    let casas = if e.tem(1) { Some(e.int(1)?) } else { None };
    let digitos = |mut v: u64| -> String {
        if v == 0 {
            return "0".into();
        }
        let mut cs = Vec::new();
        while v > 0 {
            cs.push(std::char::from_digit((v % base.raiz as u64) as u32, base.raiz).unwrap().to_ascii_uppercase());
            v /= base.raiz as u64;
        }
        cs.iter().rev().collect()
    };
    if n < 0 {
        // Negativo: as casas são ignoradas.
        return texto(digitos((n + (1i64 << base.bits)) as u64));
    }
    let s = digitos(n as u64);
    match casas {
        None => texto(s),
        Some(c) => {
            if !(1..=10).contains(&c) || s.len() as i64 > c {
                return Err(e_num());
            }
            texto(format!("{:0>w$}", s, w = c as usize))
        }
    }
}

fn de_para(args: &[Node], ev: &Evaluator<'_>, de: Base, para: Base) -> Computed {
    por_elemento(args, ev, 1, 2, |e| escrever(e, ler(e, de)?, para))
}

fn para_dec(args: &[Node], ev: &Evaluator<'_>, de: Base) -> Computed {
    por_elemento(args, ev, 1, 1, |e| numero(ler(e, de)? as f64))
}

fn de_dec(args: &[Node], ev: &Evaluator<'_>, para: Base) -> Computed {
    por_elemento(args, ev, 1, 2, |e| {
        if matches!(e.valor(0), Value::Bool(_)) {
            return Err(e_valor());
        }
        let n = e.num(0)?.trunc();
        if !n.is_finite() || n.abs() > 1e12 {
            return Err(e_num());
        }
        escrever(e, n as i64, para)
    })
}

/// `=BINADEC(núm)` / `=BIN2DEC(number)`
pub fn fn_bin2dec(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    para_dec(args, ev, BIN)
}
/// `=BINAHEX(núm; [casas])` / `=BIN2HEX(number; [places])`
pub fn fn_bin2hex(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    de_para(args, ev, BIN, HEX)
}
/// `=BINAOCT(núm; [casas])` / `=BIN2OCT(number; [places])`
pub fn fn_bin2oct(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    de_para(args, ev, BIN, OCT)
}
/// `=DECABIN(núm; [casas])` / `=DEC2BIN(number; [places])`
pub fn fn_dec2bin(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    de_dec(args, ev, BIN)
}
/// `=DECAHEX(núm; [casas])` / `=DEC2HEX(number; [places])`
pub fn fn_dec2hex(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    de_dec(args, ev, HEX)
}
/// `=DECAOCT(núm; [casas])` / `=DEC2OCT(number; [places])`
pub fn fn_dec2oct(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    de_dec(args, ev, OCT)
}
/// `=HEXABIN(núm; [casas])` / `=HEX2BIN(number; [places])`
pub fn fn_hex2bin(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    de_para(args, ev, HEX, BIN)
}
/// `=HEXADEC(núm)` / `=HEX2DEC(number)`
pub fn fn_hex2dec(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    para_dec(args, ev, HEX)
}
/// `=HEXAOCT(núm; [casas])` / `=HEX2OCT(number; [places])`
pub fn fn_hex2oct(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    de_para(args, ev, HEX, OCT)
}
/// `=OCTABIN(núm; [casas])` / `=OCT2BIN(number; [places])`
pub fn fn_oct2bin(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    de_para(args, ev, OCT, BIN)
}
/// `=OCTADEC(núm)` / `=OCT2DEC(number)`
pub fn fn_oct2dec(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    para_dec(args, ev, OCT)
}
/// `=OCTAHEX(núm; [casas])` / `=OCT2HEX(number; [places])`
pub fn fn_oct2hex(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    de_para(args, ev, OCT, HEX)
}

const LIMITE_BITS: f64 = 281_474_976_710_656.0; // 2^48

/// Inteiro não negativo abaixo de 2^48, como as funções de bits exigem.
fn bits_de(e: &Entradas<'_>, i: usize) -> Res<u64> {
    let n = e.num(i)?;
    if n < 0.0 || n.fract() != 0.0 || n >= LIMITE_BITS {
        return Err(e_num());
    }
    Ok(n as u64)
}

fn bit_binaria(args: &[Node], ev: &Evaluator<'_>, op: fn(u64, u64) -> u64) -> Computed {
    por_elemento(args, ev, 2, 2, |e| numero(op(bits_de(e, 0)?, bits_de(e, 1)?) as f64))
}

/// `=BITAND(number1; number2)`
pub fn fn_bitand(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    bit_binaria(args, ev, |a, b| a & b)
}
/// `=BITOR(number1; number2)`
pub fn fn_bitor(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    bit_binaria(args, ev, |a, b| a | b)
}
/// `=BITXOR(number1; number2)`
pub fn fn_bitxor(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    bit_binaria(args, ev, |a, b| a ^ b)
}

fn deslocar(args: &[Node], ev: &Evaluator<'_>, para_esquerda: bool) -> Computed {
    por_elemento(args, ev, 2, 2, |e| {
        let n = bits_de(e, 0)?;
        let d = e.num(1)?.trunc();
        if d.abs() > 53.0 {
            return Err(e_num());
        }
        let esq = (d >= 0.0) == para_esquerda;
        let q = d.abs() as u32;
        let r = if esq { (n as u128) << q } else { (n as u128) >> q };
        if r as f64 >= LIMITE_BITS {
            return Err(e_num());
        }
        numero(r as f64)
    })
}

/// `=DESLOCESQBIT(núm; desloc)` / `=BITLSHIFT(number; shift_amount)`
pub fn fn_bitlshift(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    deslocar(args, ev, true)
}
/// `=DESLOCDIRBIT(núm; desloc)` / `=BITRSHIFT(number; shift_amount)`
pub fn fn_bitrshift(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    deslocar(args, ev, false)
}

/// `=DELTA(núm1; [núm2])` — 1 se os números são iguais, senão 0.
pub fn fn_delta(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 1, 2, |e| {
        Ok(Value::Number(if e.num(0)? == e.num_ou(1, 0.0)? { 1.0 } else { 0.0 }))
    })
}

/// `=DEGRAU(núm; [passo])` / `=GESTEP(number; [step])` — 1 se núm >= passo.
pub fn fn_gestep(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 1, 2, |e| {
        Ok(Value::Number(if e.num(0)? >= e.num_ou(1, 0.0)? { 1.0 } else { 0.0 }))
    })
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::planilha::sheet::Sheet;

    fn calc(f: &str) -> Value {
        let mut s = Sheet::new("Plan1");
        s.set_cell_raw(0, 0, format!("={f}"), None);
        s.get_value(0, 0)
    }

    #[test]
    fn conversoes_da_documentacao() {
        assert_eq!(calc("DEC2BIN(9; 4)"), Value::Text("1001".into()));
        assert_eq!(calc("DEC2BIN(-100)"), Value::Text("1110011100".into()));
        assert_eq!(calc("DEC2HEX(-54)"), Value::Text("FFFFFFFFCA".into()));
        assert_eq!(calc("HEX2DEC(\"FFFFFFFF5B\")"), Value::Number(-165.0));
        assert_eq!(calc("BIN2DEC(1111111111)"), Value::Number(-1.0));
        assert_eq!(calc("OCT2DEC(\"7777777533\")"), Value::Number(-165.0));
        assert_eq!(calc("HEX2BIN(\"F\"; 8)"), Value::Text("00001111".into()));
        assert!(matches!(calc("DEC2BIN(512)"), Value::Error(_)));
    }

    #[test]
    fn bits_e_degraus() {
        assert_eq!(calc("BITAND(1; 5)"), Value::Number(1.0));
        assert_eq!(calc("BITOR(23; 10)"), Value::Number(31.0));
        assert_eq!(calc("BITXOR(5; 3)"), Value::Number(6.0));
        assert_eq!(calc("BITLSHIFT(4; 2)"), Value::Number(16.0));
        assert_eq!(calc("BITRSHIFT(13; 2)"), Value::Number(3.0));
        assert_eq!(calc("DELTA(5; 4)"), Value::Number(0.0));
        assert_eq!(calc("GESTEP(5; 4)"), Value::Number(1.0));
    }
}
