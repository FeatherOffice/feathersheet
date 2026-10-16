//! Financeiras de título de desconto e de vencimento único: DESC, PREÇODESC,
//! LUCRODESC, TAXAJUROS, RECEBER, OTN*, PREÇOVENC, LUCROVENC, JUROSACUMV.
//!
//! O tempo entre duas datas é a fração do ano da base escolhida (a mesma de
//! FRAÇÃOANO), como o Excel faz nessas funções. Datas truncam para o dia.

use crate::celulas::values::Computed;
use crate::formula::ast::Node;
use crate::formula::evaluator::Evaluator;
use crate::funcoes::fn_helpers::*;
use crate::funcoes::fn_date::yearfrac_core;

const MAX_SERIAL: f64 = 2_958_465.0;

fn data(args: &[Node], ev: &Evaluator<'_>, i: usize) -> Res<i64> {
    let n = arg_num(args, ev, i)?.floor();
    if !(0.0..=MAX_SERIAL).contains(&n) {
        return Err(e_num());
    }
    Ok(n as i64)
}

fn base(args: &[Node], ev: &Evaluator<'_>, i: usize) -> Res<i64> {
    let b = if has(args, i) { para_int(arg_num(args, ev, i)?)? } else { 0 };
    if !(0..=4).contains(&b) {
        return Err(e_num());
    }
    Ok(b)
}

/// Fração de ano entre duas datas (já em ordem).
fn frac(de: i64, ate: i64, base: i64) -> Res<f64> {
    yearfrac_core(de, ate, base)
}

/// Liquidação e vencimento nos argumentos `i` e `i+1`: liquidação < vencimento.
fn liq_venc(args: &[Node], ev: &Evaluator<'_>, i: usize) -> Res<(i64, i64)> {
    let (s, m) = (data(args, ev, i)?, data(args, ev, i + 1)?);
    if s >= m {
        return Err(e_num());
    }
    Ok((s, m))
}

fn positivo(n: f64) -> Res<f64> {
    if n <= 0.0 { Err(e_num()) } else { Ok(n) }
}

/// `=DESC(liquidação; vencimento; preço; resgate; [base])` / `=DISC(settlement, maturity, pr, redemption, [basis])`
pub fn fn_disc(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    executar(args, 4, 5, || {
            let (s, m) = liq_venc(args, ev, 0)?;
            let pr = positivo(arg_num(args, ev, 2)?)?;
            let red = positivo(arg_num(args, ev, 3)?)?;
            let b = base(args, ev, 4)?;
            numero((red - pr) / red / frac(s, m, b)?)
    })
}

/// `=PREÇODESC(liquidação; vencimento; desconto; resgate; [base])` / `=PRICEDISC(settlement, maturity, discount, redemption, [basis])`
pub fn fn_pricedisc(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    executar(args, 4, 5, || {
            let (s, m) = liq_venc(args, ev, 0)?;
            let desc = positivo(arg_num(args, ev, 2)?)?;
            let red = positivo(arg_num(args, ev, 3)?)?;
            let b = base(args, ev, 4)?;
            numero(red - desc * red * frac(s, m, b)?)
    })
}

/// `=LUCRODESC(liquidação; vencimento; preço; resgate; [base])` / `=YIELDDISC(settlement, maturity, pr, redemption, [basis])`
pub fn fn_yielddisc(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    executar(args, 4, 5, || {
            let (s, m) = liq_venc(args, ev, 0)?;
            let pr = positivo(arg_num(args, ev, 2)?)?;
            let red = positivo(arg_num(args, ev, 3)?)?;
            let b = base(args, ev, 4)?;
            numero((red - pr) / pr / frac(s, m, b)?)
    })
}

/// `=TAXAJUROS(liquidação; vencimento; investimento; resgate; [base])` / `=INTRATE(settlement, maturity, investment, redemption, [basis])`
pub fn fn_intrate(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    executar(args, 4, 5, || {
            let (s, m) = liq_venc(args, ev, 0)?;
            let inv = positivo(arg_num(args, ev, 2)?)?;
            let red = positivo(arg_num(args, ev, 3)?)?;
            let b = base(args, ev, 4)?;
            numero((red - inv) / inv / frac(s, m, b)?)
    })
}

/// `=RECEBER(liquidação; vencimento; investimento; desconto; [base])` / `=RECEIVED(settlement, maturity, investment, discount, [basis])`
pub fn fn_received(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    executar(args, 4, 5, || {
            let (s, m) = liq_venc(args, ev, 0)?;
            let inv = positivo(arg_num(args, ev, 2)?)?;
            let desc = positivo(arg_num(args, ev, 3)?)?;
            let b = base(args, ev, 4)?;
            let den = 1.0 - desc * frac(s, m, b)?;
            if den <= 0.0 {
                return Err(e_num());
            }
            numero(inv / den)
    })
}

/// OTN: vencimento até um ano depois da liquidação; dias reais.
fn otn_dias(args: &[Node], ev: &Evaluator<'_>) -> Res<f64> {
    let (s, m) = liq_venc(args, ev, 0)?;
    if m - s > 365 {
        return Err(e_num());
    }
    Ok((m - s) as f64)
}

/// `=OTNVALOR(liquidação; vencimento; desconto)` / `=TBILLPRICE(settlement, maturity, discount)`
pub fn fn_tbillprice(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    executar(args, 3, 3, || {
            let dias = otn_dias(args, ev)?;
            let desc = positivo(arg_num(args, ev, 2)?)?;
            let p = 100.0 * (1.0 - desc * dias / 360.0);
            if p <= 0.0 {
                return Err(e_num());
            }
            numero(p)
    })
}

/// `=OTNLUCRO(liquidação; vencimento; preço)` / `=TBILLYIELD(settlement, maturity, pr)`
pub fn fn_tbillyield(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    executar(args, 3, 3, || {
            let dias = otn_dias(args, ev)?;
            let pr = positivo(arg_num(args, ev, 2)?)?;
            numero((100.0 - pr) / pr * 360.0 / dias)
    })
}

/// `=OTN(liquidação; vencimento; desconto)` / `=TBILLEQ(settlement, maturity, discount)`
pub fn fn_tbilleq(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    executar(args, 3, 3, || {
            let dias = otn_dias(args, ev)?;
            let desc = positivo(arg_num(args, ev, 2)?)?;
            if desc * dias / 360.0 >= 1.0 {
                return Err(e_num());
            }
            if dias <= 182.0 {
                return numero(365.0 * desc / (360.0 - desc * dias));
            }
            let p = 100.0 * (1.0 - desc * dias / 360.0);
            let (ca, cb, cc) = (dias / 730.0 - 0.25, dias / 365.0, (p - 100.0) / p);
            let delta = cb * cb - 4.0 * ca * cc;
            if delta < 0.0 {
                return Err(e_num());
            }
            numero((-cb + delta.sqrt()) / (2.0 * ca))
    })
}

/// `=PREÇOVENC(liquidação; vencimento; emissão; taxa; lucro; [base])` / `=PRICEMAT(settlement, maturity, issue, rate, yld, [basis])`
pub fn fn_pricemat(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    executar(args, 5, 6, || {
            let (s, m) = liq_venc(args, ev, 0)?;
            let emissao = data(args, ev, 2)?;
            let (taxa, lucro) = (arg_num(args, ev, 3)?, arg_num(args, ev, 4)?);
            let b = base(args, ev, 5)?;
            if emissao > s || taxa < 0.0 || lucro < 0.0 {
                return Err(e_num());
            }
            let (f_em_venc, f_liq_venc, f_em_liq) = (frac(emissao, m, b)?, frac(s, m, b)?, frac(emissao, s, b)?);
            numero((100.0 + f_em_venc * taxa * 100.0) / (1.0 + f_liq_venc * lucro) - f_em_liq * taxa * 100.0)
    })
}

/// `=LUCROVENC(liquidação; vencimento; emissão; taxa; preço; [base])` / `=YIELDMAT(settlement, maturity, issue, rate, pr, [basis])`
pub fn fn_yieldmat(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    executar(args, 5, 6, || {
            let (s, m) = liq_venc(args, ev, 0)?;
            let emissao = data(args, ev, 2)?;
            let (taxa, pr) = (arg_num(args, ev, 3)?, positivo(arg_num(args, ev, 4)?)?);
            let b = base(args, ev, 5)?;
            if emissao > s || taxa < 0.0 {
                return Err(e_num());
            }
            let (f_em_venc, f_liq_venc, f_em_liq) = (frac(emissao, m, b)?, frac(s, m, b)?, frac(emissao, s, b)?);
            let preco_corr = pr / 100.0 + f_em_liq * taxa;
            numero(((1.0 + f_em_venc * taxa) - preco_corr) / preco_corr / f_liq_venc)
    })
}

/// `=JUROSACUMV(emissão; liquidação; taxa; valor_nominal; [base])` / `=ACCRINTM(issue, settlement, rate, par, [basis])`
pub fn fn_accrintm(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    executar(args, 4, 5, || {
            let (emissao, s) = (data(args, ev, 0)?, data(args, ev, 1)?);
            let taxa = positivo(arg_num(args, ev, 2)?)?;
            let par = positivo(arg_num(args, ev, 3)?)?;
            let b = base(args, ev, 4)?;
            if emissao >= s {
                return Err(e_num());
            }
            numero(par * taxa * frac(emissao, s, b)?)
    })
}

#[cfg(test)]
mod testes {
    use crate::celulas::values::ErrorCode;
    use crate::funcoes::fn_helpers::apoio_teste::*;

    fn perto(v: crate::celulas::values::Value, esperado: f64, tol: f64) {
        match v {
            crate::celulas::values::Value::Number(n) => {
                assert!((n - esperado).abs() < tol, "obtido {n}, esperado {esperado}")
            }
            o => panic!("esperava número, veio {o:?}"),
        }
    }

    #[test]
    fn desconto_e_taxa() {
        perto(calc("DISC(DATE(2018;1;25);DATE(2018;6;15);97,975;100;1)"), 0.052420213, 1e-8);
        perto(calc("DESC(DATE(2018;1;25);DATE(2018;6;15);97,975;100;1)"), 0.052420213, 1e-8);
        perto(calc("PRICEDISC(DATE(2008;2;16);DATE(2008;3;1);0,0525;100;2)"), 99.7958333, 1e-6);
        perto(calc("PREÇODESC(DATE(2008;2;16);DATE(2008;3;1);0,0525;100;2)"), 99.7958333, 1e-6);
        perto(calc("YIELDDISC(DATE(2008;2;16);DATE(2008;3;1);99,795;100;2)"), 0.052823, 1e-6);
        perto(calc("INTRATE(DATE(2008;2;15);DATE(2008;5;15);1000000;1014420;2)"), 0.05768, 1e-5);
        perto(calc("RECEIVED(DATE(2008;2;15);DATE(2008;5;15);1000000;0,0575;2)"), 1014584.654, 1e-2);
        is_err(calc("DISC(DATE(2018;6;15);DATE(2018;1;25);97;100)"), ErrorCode::Num);
        is_err(calc("DISC(DATE(2018;1;25);DATE(2018;6;15);0;100)"), ErrorCode::Num);
        is_err(calc("DISC(DATE(2018;1;25);DATE(2018;6;15);97;100;7)"), ErrorCode::Num);
        is_err(calc("DISC(\"x\";DATE(2018;6;15);97;100)"), ErrorCode::Value);
    }

    #[test]
    fn letras_do_tesouro() {
        perto(calc("TBILLPRICE(DATE(2008;3;31);DATE(2008;6;1);0,09)"), 98.45, 1e-9);
        perto(calc("OTNVALOR(DATE(2008;3;31);DATE(2008;6;1);0,09)"), 98.45, 1e-9);
        perto(calc("TBILLYIELD(DATE(2008;3;31);DATE(2008;6;1);98,45)"), 0.0914169, 1e-6);
        perto(calc("OTNLUCRO(DATE(2008;3;31);DATE(2008;6;1);98,45)"), 0.0914169, 1e-6);
        perto(calc("TBILLEQ(DATE(2008;3;31);DATE(2008;6;1);0,0914)"), 0.094151, 1e-6);
        perto(calc("OTN(DATE(2008;3;31);DATE(2008;6;1);0,0914)"), 0.094151, 1e-6);
        // Mais de 182 dias: a equação do segundo grau.
        perto(calc("TBILLEQ(DATE(2008;1;1);DATE(2008;12;1);0,05)"), 0.052535, 1e-5);
        is_err(calc("TBILLPRICE(DATE(2008;3;31);DATE(2009;6;1);0,09)"), ErrorCode::Num);
        is_err(calc("TBILLEQ(DATE(2008;3;31);DATE(2008;6;1);-1)"), ErrorCode::Num);
    }

    #[test]
    fn titulo_de_vencimento_unico() {
        perto(calc("PRICEMAT(DATE(2008;2;15);DATE(2008;4;13);DATE(2007;11;11);0,061;0,061;0)"), 99.98449888, 1e-6);
        perto(calc("PREÇOVENC(DATE(2008;2;15);DATE(2008;4;13);DATE(2007;11;11);0,061;0,061;0)"), 99.98449888, 1e-6);
        perto(calc("YIELDMAT(DATE(2008;3;15);DATE(2008;11;3);DATE(2007;11;8);0,0625;100,0123;0)"), 0.060954, 1e-6);
        perto(calc("LUCROVENC(DATE(2008;3;15);DATE(2008;11;3);DATE(2007;11;8);0,0625;100,0123;0)"), 0.060954, 1e-6);
        perto(calc("ACCRINTM(DATE(2008;4;1);DATE(2008;6;15);0,1;1000;3)"), 20.54794521, 1e-6);
        perto(calc("JUROSACUMV(DATE(2008;4;1);DATE(2008;6;15);0,1;1000;3)"), 20.54794521, 1e-6);
        is_err(calc("PRICEMAT(DATE(2008;2;15);DATE(2008;4;13);DATE(2008;3;1);0,061;0,061)"), ErrorCode::Num);
        is_err(calc("ACCRINTM(DATE(2008;6;15);DATE(2008;4;1);0,1;1000)"), ErrorCode::Num);
        is_err(calc("YIELDMAT(DATE(2008;3;15);DATE(2008;11;3);DATE(2007;11;8);0,0625;0)"), ErrorCode::Num);
    }
}
