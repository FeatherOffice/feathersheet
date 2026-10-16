//! Funções financeiras em Rust: PGTO, VF, VP, NPER, IPGTO, PPGTO, ÉPGTO, DPD, SDA, BD, BDD, BDV, EFETIVA, NOMINAL, MOEDADEC, MOEDAFRA, TAXAJURO, DURAÇÃOP, VPL, TAXA, PGTOJURACUM, PGTOCAPACUM, XVPL, XTIR, TIR, MTIR, VFPLANO.
//!
//! Convenção de sinal do Excel: dinheiro que sai é negativo, dinheiro que
//! entra é positivo. `tipo` 0 (ou omitido) quer dizer pagamento no fim do
//! período; qualquer outro valor, no começo.

use crate::celulas::values::{Computed, Value};
use crate::formula::ast::Node;
use crate::formula::evaluator::Evaluator;
use crate::funcoes::fn_helpers::{
    arity, concluir, e_div0, e_num, e_valor, matriz_de, num, numero, numeros_de,
    por_elemento, Entradas, Res,
};

// ---- contas básicas do valor do dinheiro no tempo ----

/// `tipo` lido como no Excel: zero é fim do período, o resto é começo.
fn tipo_de(e: &Entradas<'_>, i: usize) -> Res<f64> {
    Ok(if e.num_ou(i, 0.0)? != 0.0 { 1.0 } else { 0.0 })
}

/// Pagamento constante que zera a equação do valor no tempo.
pub(crate) fn pgto(taxa: f64, nper: f64, vp: f64, vf: f64, tipo: f64) -> f64 {
    if taxa == 0.0 {
        return -(vp + vf) / nper;
    }
    let q = (1.0 + taxa).powf(nper);
    -taxa * (vf + vp * q) / ((1.0 + taxa * tipo) * (q - 1.0))
}

/// Valor futuro de um saldo `vp` com pagamentos `pmt`.
pub(crate) fn vf(taxa: f64, nper: f64, pmt: f64, vp: f64, tipo: f64) -> f64 {
    if taxa == 0.0 {
        return -(vp + pmt * nper);
    }
    let q = (1.0 + taxa).powf(nper);
    -(vp * q + pmt * (1.0 + taxa * tipo) * (q - 1.0) / taxa)
}

/// Valor presente de pagamentos `pmt` e de um valor final `vf`.
pub(crate) fn vp(taxa: f64, nper: f64, pmt: f64, vf: f64, tipo: f64) -> f64 {
    if taxa == 0.0 {
        return -(vf + pmt * nper);
    }
    let q = (1.0 + taxa).powf(nper);
    -(vf + pmt * (1.0 + taxa * tipo) * (q - 1.0) / taxa) / q
}

/// Juros pagos no período `per`: a taxa sobre o saldo devedor do começo dele.
pub(crate) fn juros_do_periodo(taxa: f64, per: f64, nper: f64, vp_: f64, vf_: f64, tipo: f64) -> f64 {
    if taxa == 0.0 {
        return 0.0;
    }
    let pmt = pgto(taxa, nper, vp_, vf_, tipo);
    let saldo = if tipo == 1.0 {
        // Pagando no começo, o primeiro pagamento sai antes de render juros.
        if per == 1.0 {
            return 0.0;
        }
        vf(taxa, per - 2.0, pmt, vp_, 1.0) - pmt
    } else {
        vf(taxa, per - 1.0, pmt, vp_, 0.0)
    };
    saldo * taxa
}

/// Newton–Raphson: devolve a raiz quando o passo fica menor que `tol`
/// em no máximo `voltas` iterações; senão `None` (a planilha mostra #NÚM!).
fn newton(f: &impl Fn(f64) -> (f64, f64), chute: f64, voltas: usize, tol: f64) -> Option<f64> {
    let mut x = chute;
    for _ in 0..voltas {
        let (y, dy) = f(x);
        if !y.is_finite() || !dy.is_finite() || dy == 0.0 {
            return None;
        }
        let novo = x - y / dy;
        if !novo.is_finite() {
            return None;
        }
        if (novo - x).abs() < tol {
            return Some(novo);
        }
        x = novo;
    }
    None
}

/// Raiz de uma equação de taxa (só vale taxa > −1).
///
/// Primeiro o Newton a partir da estimativa, como o Excel (`voltas`
/// iterações, passo menor que `tol`). Quando ele escapa — estimativa longe,
/// prazo longo que deixa a curva íngreme —, procura uma troca de sinal numa
/// malha de taxas, a mais perto da estimativa, e fecha por bisseção. Sem
/// troca de sinal não há raiz e a planilha mostra #NÚM!.
fn raiz_de_taxa(f: impl Fn(f64) -> (f64, f64), chute: f64, voltas: usize, tol: f64) -> Option<f64> {
    if let Some(r) = newton(&f, chute, voltas, tol) {
        if r > -1.0 {
            return Some(r);
        }
    }
    let mut malha: Vec<f64> = (1..=8).map(|j| -1.0 + 10f64.powi(-j)).collect();
    malha.extend((0..=190).map(|k| -0.9 + k as f64 * 0.01));
    malha.extend((3..=20).map(|k| k as f64 * 0.5));
    malha.extend([20.0, 50.0, 100.0, 1000.0]);
    malha.sort_by(|a, b| a.total_cmp(b));
    let g = |r: f64| f(r).0;
    let mut melhor: Option<(f64, f64)> = None;
    for par in malha.windows(2) {
        let (a, b) = (par[0], par[1]);
        let (ga, gb) = (g(a), g(b));
        if !ga.is_finite() || !gb.is_finite() || ga.signum() == gb.signum() && ga != 0.0 {
            continue;
        }
        let distancia = |(x, y): (f64, f64)| (chute - x).abs().min((chute - y).abs());
        if melhor.is_none_or(|m| distancia((a, b)) < distancia(m)) {
            melhor = Some((a, b));
        }
    }
    let (mut a, mut b) = melhor?;
    let mut ga = g(a);
    if ga == 0.0 {
        return Some(a);
    }
    if g(b) == 0.0 {
        return Some(b);
    }
    for _ in 0..200 {
        let m = 0.5 * (a + b);
        let gm = g(m);
        if gm == 0.0 || (b - a).abs() < 1e-15 * m.abs().max(1e-10) {
            return Some(m);
        }
        if gm.signum() == ga.signum() {
            a = m;
            ga = gm;
        } else {
            b = m;
        }
    }
    Some(0.5 * (a + b))
}

/// `=PGTO(taxa; nper; vp; [vf]; [tipo])` / `=PMT(rate; nper; pv; [fv]; [type])` — pagamento periódico constante.
pub fn fn_pmt(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 3, 5, |e| {
        let (taxa, nper, vp_) = (e.num(0)?, e.num(1)?, e.num(2)?);
        let (vf_, tipo) = (e.num_ou(3, 0.0)?, tipo_de(e, 4)?);
        if nper == 0.0 {
            return Err(e_num());
        }
        numero(pgto(taxa, nper, vp_, vf_, tipo))
    })
}

/// `=VF(taxa; nper; pgto; [vp]; [tipo])` / `=FV(rate; nper; pmt; [pv]; [type])` — valor futuro de um investimento.
pub fn fn_fv(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 3, 5, |e| {
        let (taxa, nper, pmt) = (e.num(0)?, e.num(1)?, e.num(2)?);
        let (vp_, tipo) = (e.num_ou(3, 0.0)?, tipo_de(e, 4)?);
        numero(vf(taxa, nper, pmt, vp_, tipo))
    })
}

/// `=VP(taxa; nper; pgto; [vf]; [tipo])` / `=PV(rate; nper; pmt; [fv]; [type])` — valor presente de um investimento.
pub fn fn_pv(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 3, 5, |e| {
        let (taxa, nper, pmt) = (e.num(0)?, e.num(1)?, e.num(2)?);
        let (vf_, tipo) = (e.num_ou(3, 0.0)?, tipo_de(e, 4)?);
        numero(vp(taxa, nper, pmt, vf_, tipo))
    })
}

/// `=NPER(taxa; pgto; vp; [vf]; [tipo])` / `=NPER(rate; pmt; pv; [fv]; [type])` — quantos períodos até quitar.
pub fn fn_nper(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 3, 5, |e| {
        let (taxa, pmt, vp_) = (e.num(0)?, e.num(1)?, e.num(2)?);
        let (vf_, tipo) = (e.num_ou(3, 0.0)?, tipo_de(e, 4)?);
        if taxa == 0.0 {
            if pmt == 0.0 {
                return Err(e_num());
            }
            return numero(-(vp_ + vf_) / pmt);
        }
        if taxa <= -1.0 {
            return Err(e_num());
        }
        let p = pmt * (1.0 + taxa * tipo);
        let razao = (p - vf_ * taxa) / (p + vp_ * taxa);
        if razao <= 0.0 {
            return Err(e_num());
        }
        numero(razao.ln() / (1.0 + taxa).ln())
    })
}

/// `=IPGTO(taxa; per; nper; vp; [vf]; [tipo])` / `=IPMT(rate; per; nper; pv; [fv]; [type])` — parte de juros do pagamento do período.
pub fn fn_ipmt(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 4, 6, |e| {
        let (taxa, per, nper, vp_) = (e.num(0)?, e.num(1)?, e.num(2)?, e.num(3)?);
        let (vf_, tipo) = (e.num_ou(4, 0.0)?, tipo_de(e, 5)?);
        if per < 1.0 || per > nper {
            return Err(e_num());
        }
        numero(juros_do_periodo(taxa, per, nper, vp_, vf_, tipo))
    })
}

/// `=PPGTO(taxa; per; nper; vp; [vf]; [tipo])` / `=PPMT(rate; per; nper; pv; [fv]; [type])` — parte de amortização do pagamento do período.
pub fn fn_ppmt(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 4, 6, |e| {
        let (taxa, per, nper, vp_) = (e.num(0)?, e.num(1)?, e.num(2)?, e.num(3)?);
        let (vf_, tipo) = (e.num_ou(4, 0.0)?, tipo_de(e, 5)?);
        if per < 1.0 || per > nper {
            return Err(e_num());
        }
        numero(pgto(taxa, nper, vp_, vf_, tipo) - juros_do_periodo(taxa, per, nper, vp_, vf_, tipo))
    })
}

/// `=ÉPGTO(taxa; per; nper; vp)` / `=ISPMT(rate; per; nper; pv)` — juros do período com amortização constante.
pub fn fn_ispmt(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 4, 4, |e| {
        let (taxa, per, nper, vp_) = (e.num(0)?, e.num(1)?, e.num(2)?, e.num(3)?);
        if nper == 0.0 {
            return Err(e_div0());
        }
        numero(vp_ * taxa * (per / nper - 1.0))
    })
}

// ---- depreciação ----

/// `=DPD(custo; recuperação; vida_útil)` / `=SLN(cost; salvage; life)` — depreciação linear por período.
pub fn fn_sln(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 3, 3, |e| {
        let (custo, resto, vida) = (e.num(0)?, e.num(1)?, e.num(2)?);
        if vida == 0.0 {
            return Err(e_div0());
        }
        numero((custo - resto) / vida)
    })
}

/// `=SDA(custo; recuperação; vida_útil; per)` / `=SYD(cost; salvage; life; per)` — soma dos dígitos dos anos.
pub fn fn_syd(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 4, 4, |e| {
        let (custo, resto, vida, per) = (e.num(0)?, e.num(1)?, e.num(2)?, e.num(3)?);
        if resto < 0.0 || vida <= 0.0 || per <= 0.0 || per > vida {
            return Err(e_num());
        }
        numero((custo - resto) * (vida - per + 1.0) * 2.0 / (vida * (vida + 1.0)))
    })
}

/// `=BD(custo; recuperação; vida_útil; período; [mês])` / `=DB(cost; salvage; life; period; [month])` — saldo decrescente fixo.
///
/// A taxa é `1 − (recuperação/custo)^(1/vida)` arredondada a três casas; o
/// primeiro ano só conta os `mês` meses e o que falta vai para o ano
/// `vida + 1`.
pub fn fn_db(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 4, 5, |e| {
        let (custo, resto, vida) = (e.num(0)?, e.num(1)?, e.num(2)?);
        let periodo = e.num(3)?.trunc();
        let meses = e.num_ou(4, 12.0)?.trunc();
        if custo < 0.0 || resto < 0.0 || vida <= 0.0 || periodo <= 0.0 {
            return Err(e_num());
        }
        if !(1.0..=12.0).contains(&meses) || periodo > vida + 1.0 || (periodo > vida && meses == 12.0) {
            return Err(e_num());
        }
        if custo == 0.0 {
            return numero(0.0);
        }
        let taxa = ((1.0 - (resto / custo).powf(1.0 / vida)) * 1000.0).round() / 1000.0;
        let primeiro = custo * taxa * meses / 12.0;
        if periodo == 1.0 {
            return numero(primeiro);
        }
        let mut acumulado = primeiro;
        let mut deste = primeiro;
        let mut p = 2.0;
        while p <= periodo {
            deste = if p > vida {
                (custo - acumulado) * taxa * (12.0 - meses) / 12.0
            } else {
                (custo - acumulado) * taxa
            };
            acumulado += deste;
            p += 1.0;
        }
        numero(deste)
    })
}

/// Depreciação de saldo decrescente do período `p` (pode ser fracionário):
/// o valor contábil cai à razão `fator/vida` por período e nunca fica abaixo
/// da recuperação.
fn saldo_decrescente(custo: f64, resto: f64, vida: f64, p: f64, fator: f64) -> f64 {
    let taxa = (fator / vida).min(1.0);
    let antes = if taxa >= 1.0 {
        if p <= 1.0 { custo } else { 0.0 }
    } else {
        custo * (1.0 - taxa).powf(p - 1.0)
    };
    let depois = if taxa >= 1.0 { 0.0 } else { custo * (1.0 - taxa).powf(p) };
    let d = if depois < resto { antes - resto } else { antes - depois };
    d.max(0.0)
}

/// `=BDD(custo; recuperação; vida_útil; período; [fator])` / `=DDB(cost; salvage; life; period; [factor])` — saldo decrescente duplo (ou com outro fator).
pub fn fn_ddb(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 4, 5, |e| {
        let (custo, resto, vida, periodo) = (e.num(0)?, e.num(1)?, e.num(2)?, e.num(3)?);
        let fator = e.num_ou(4, 2.0)?;
        if custo < 0.0 || resto < 0.0 || vida <= 0.0 || periodo <= 0.0 || fator <= 0.0 || periodo > vida {
            return Err(e_num());
        }
        numero(saldo_decrescente(custo, resto, vida, periodo, fator))
    })
}

/// A depreciação de cada período inteiro (1, 2, …, `ate`) pelo saldo
/// decrescente, trocando para a linear quando ela passa a dar mais (salvo
/// `sem_troca`).
fn tabela_vdb(custo: f64, resto: f64, vida: f64, fator: f64, sem_troca: bool, ate: usize) -> Vec<f64> {
    let mut saida = Vec::with_capacity(ate);
    let mut livro = custo;
    let mut linear: Option<f64> = None;
    for i in 0..ate {
        let d = if let Some(l) = linear {
            l.min((livro - resto).max(0.0))
        } else {
            let ddb = (livro * fator / vida).min((livro - resto).max(0.0)).max(0.0);
            let restante = vida - i as f64;
            let sl = if restante > 0.0 { ((livro - resto) / restante).max(0.0) } else { 0.0 };
            if !sem_troca && sl > ddb {
                linear = Some(sl);
                sl
            } else {
                ddb
            }
        };
        saida.push(d);
        livro -= d;
    }
    saida
}

/// `=BDV(custo; recuperação; vida_útil; início; fim; [fator]; [sem_mudança])` / `=VDB(cost; salvage; life; start; end; [factor]; [no_switch])` — depreciação acumulada entre dois períodos, com troca para a linear.
///
/// Dentro de um período a depreciação corre de forma linear, então
/// `início`/`fim` fracionários pegam a fração do período.
pub fn fn_vdb(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 5, 7, |e| {
        let (custo, resto, vida, ini, fim) = (e.num(0)?, e.num(1)?, e.num(2)?, e.num(3)?, e.num(4)?);
        let fator = e.num_ou(5, 2.0)?;
        let sem_troca = e.logico_ou(6, false)?;
        if custo < 0.0 || resto < 0.0 || vida <= 0.0 || fator <= 0.0 || ini < 0.0 || fim < ini || fim > vida {
            return Err(e_num());
        }
        let ate = fim.ceil() as usize;
        let tabela = tabela_vdb(custo, resto, vida, fator, sem_troca, ate);
        let acumulado = |x: f64| -> f64 {
            let inteiro = x.floor() as usize;
            let mut s: f64 = tabela[..inteiro.min(tabela.len())].iter().sum();
            let frac = x - x.floor();
            if frac > 0.0 {
                s += frac * tabela.get(inteiro).copied().unwrap_or(0.0);
            }
            s
        };
        numero(acumulado(fim) - acumulado(ini))
    })
}

// ---- taxas e cotações ----

/// `=EFETIVA(taxa_nominal; nper_ano)` / `=EFFECT(nominal_rate; npery)` — taxa efetiva anual.
pub fn fn_effect(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 2, 2, |e| {
        let (nominal, n) = (e.num(0)?, e.num(1)?.trunc());
        if nominal <= 0.0 || n < 1.0 {
            return Err(e_num());
        }
        numero((1.0 + nominal / n).powf(n) - 1.0)
    })
}

/// `=NOMINAL(taxa_efetiva; nper_ano)` / `=NOMINAL(effect_rate; npery)` — taxa nominal anual.
pub fn fn_nominal(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 2, 2, |e| {
        let (efetiva, n) = (e.num(0)?, e.num(1)?.trunc());
        if efetiva <= 0.0 || n < 1.0 {
            return Err(e_num());
        }
        numero(n * ((1.0 + efetiva).powf(1.0 / n) - 1.0))
    })
}

/// A fração do MOEDADEC/MOEDAFRA: inteiro ≥ 1; negativa dá #NÚM!, entre 0 e 1 dá #DIV/0!.
fn fracao_de(e: &Entradas<'_>, i: usize) -> Res<f64> {
    let f = e.num(i)?.trunc();
    if f < 0.0 {
        return Err(e_num());
    }
    if f < 1.0 {
        return Err(e_div0());
    }
    Ok(f)
}

/// `=MOEDADEC(moeda_fracionária; fração)` / `=DOLLARDE(fractional_dollar; fraction)` — cotação fracionária em decimal.
pub fn fn_dollarde(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 2, 2, |e| {
        let x = e.num(0)?;
        let f = fracao_de(e, 1)?;
        let escala = 10f64.powf(f.log10().ceil());
        let inteiro = x.abs().trunc();
        let resto = x.abs() - inteiro;
        numero(x.signum() * (inteiro + resto * escala / f))
    })
}

/// `=MOEDAFRA(moeda_decimal; fração)` / `=DOLLARFR(decimal_dollar; fraction)` — cotação decimal em fracionária.
pub fn fn_dollarfr(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 2, 2, |e| {
        let x = e.num(0)?;
        let f = fracao_de(e, 1)?;
        let escala = 10f64.powf(f.log10().ceil());
        let inteiro = x.abs().trunc();
        let resto = x.abs() - inteiro;
        numero(x.signum() * (inteiro + resto * f / escala))
    })
}

/// `=TAXAJURO(nper; vp; vf)` / `=RRI(nper; pv; fv)` — taxa equivalente de crescimento.
pub fn fn_rri(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 3, 3, |e| {
        let (n, vp_, vf_) = (e.num(0)?, e.num(1)?, e.num(2)?);
        if n <= 0.0 || vp_ == 0.0 {
            return Err(e_num());
        }
        numero((vf_ / vp_).powf(1.0 / n) - 1.0)
    })
}

/// `=DURAÇÃOP(taxa; vp; vf)` / `=PDURATION(rate; pv; fv)` — períodos para `vp` virar `vf`.
pub fn fn_pduration(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 3, 3, |e| {
        let (taxa, vp_, vf_) = (e.num(0)?, e.num(1)?, e.num(2)?);
        if taxa <= 0.0 || vp_ <= 0.0 || vf_ <= 0.0 {
            return Err(e_num());
        }
        numero((vf_.ln() - vp_.ln()) / (1.0 + taxa).ln())
    })
}

// ---- fluxos de caixa ----

/// Valor presente de fluxos igualmente espaçados, o primeiro descontado
/// um período (`desloc` = 1) ou nenhum (`desloc` = 0).
fn vpl(taxa: f64, fluxos: &[f64], desloc: f64) -> f64 {
    let base = 1.0 + taxa;
    fluxos.iter().enumerate().map(|(i, v)| v / base.powf(i as f64 + desloc)).sum()
}

/// `=VPL(taxa; valor1; [valor2]; ...)` / `=NPV(rate; value1; [value2]; ...)` — valor presente líquido.
pub fn fn_npv(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    if let Some(e) = arity(args, 2, 255) {
        return Computed::Scalar(Value::Error(e));
    }
    concluir((|| {
        let taxa = ev.number(&args[0])?;
        if taxa == -1.0 {
            return Err(e_div0());
        }
        // Solto, texto que não é número dá #VALOR!; dentro de intervalo é ignorado.
        let mut fluxos = Vec::new();
        for a in &args[1..] {
            let mut ruim = None;
            let erro = ev.for_each_value(a, &mut |v, de_colecao| match v {
                Value::Number(n) => fluxos.push(*n),
                _ if de_colecao => {}
                Value::Empty => {}
                outro => match num(outro) {
                    Ok(n) => fluxos.push(n),
                    Err(e) => ruim = Some(e),
                },
            });
            if let Some(e) = erro.or(ruim) {
                return Err(e);
            }
        }
        numero(vpl(taxa, &fluxos, 1.0))
    })())
}

/// `=TAXA(nper; pgto; vp; [vf]; [tipo]; [estimativa])` / `=RATE(nper; pmt; pv; [fv]; [type]; [guess])` — taxa por período.
///
/// Newton a partir da estimativa (10% se omitida), 20 iterações com passo
/// abaixo de 1e-7 como o Excel; se escapar, busca a raiz por bisseção.
pub fn fn_rate(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 3, 6, |e| {
        let (n, pmt, vp_) = (e.num(0)?, e.num(1)?, e.num(2)?);
        let (vf_, tipo) = (e.num_ou(3, 0.0)?, tipo_de(e, 4)?);
        let chute = e.num_ou(5, 0.1)?;
        if n <= 0.0 {
            return Err(e_num());
        }
        // A equação do valor no tempo dividida por (1+r)^nper: com nper
        // grande ela fica bem comportada e o Newton converge depressa.
        let f = |r: f64| -> (f64, f64) {
            if r.abs() < 1e-10 {
                let y = vp_ + pmt * n + vf_;
                let dy = pmt * (tipo * n - n * (n + 1.0) / 2.0) - vf_ * n;
                return (y, dy);
            }
            let v = (1.0 + r).powf(-n);
            let dv = -n * (1.0 + r).powf(-n - 1.0);
            let a = (1.0 - v) / r;
            let da = (-dv * r - (1.0 - v)) / (r * r);
            let y = vp_ + pmt * (1.0 + r * tipo) * a + vf_ * v;
            let dy = pmt * (tipo * a + (1.0 + r * tipo) * da) + vf_ * dv;
            (y, dy)
        };
        match raiz_de_taxa(f, chute, 20, 1e-7) {
            Some(r) => numero(r),
            None => Err(e_num()),
        }
    })
}

/// O que PGTOJURACUM e PGTOCAPACUM conferem e somam.
fn acumulado(e: &Entradas<'_>, principal: bool) -> Res<Value> {
    let (taxa, nper, vp_) = (e.num(0)?, e.num(1)?.trunc(), e.num(2)?);
    let (ini, fim) = (e.num(3)?.trunc(), e.num(4)?.trunc());
    let tipo = e.num(5)?;
    if taxa <= 0.0 || nper <= 0.0 || vp_ <= 0.0 {
        return Err(e_num());
    }
    if ini < 1.0 || fim < 1.0 || ini > fim || fim > nper || (tipo != 0.0 && tipo != 1.0) {
        return Err(e_num());
    }
    let pmt = pgto(taxa, nper, vp_, 0.0, tipo);
    let mut soma = 0.0;
    let mut p = ini;
    while p <= fim {
        let juros = juros_do_periodo(taxa, p, nper, vp_, 0.0, tipo);
        soma += if principal { pmt - juros } else { juros };
        p += 1.0;
    }
    numero(soma)
}

/// `=PGTOJURACUM(taxa; nper; vp; início; fim; tipo)` / `=CUMIPMT(rate; nper; pv; start_period; end_period; type)` — juros acumulados entre dois períodos.
pub fn fn_cumipmt(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 6, 6, |e| acumulado(e, false))
}

/// `=PGTOCAPACUM(taxa; nper; vp; início; fim; tipo)` / `=CUMPRINC(rate; nper; pv; start_period; end_period; type)` — amortização acumulada entre dois períodos.
pub fn fn_cumprinc(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    por_elemento(args, ev, 6, 6, |e| acumulado(e, true))
}

/// Os fluxos e as datas de XVPL/XTIR: mesmo tamanho, números, datas
/// truncadas e nenhuma antes da primeira.
fn fluxos_datados(ev: &Evaluator<'_>, valores: &Node, datas: &Node) -> Res<(Vec<f64>, Vec<f64>)> {
    let mv = matriz_de(ev, valores)?;
    let md = matriz_de(ev, datas)?;
    if mv.size() != md.size() || mv.size() == 0 {
        return Err(e_num());
    }
    let mut v = Vec::with_capacity(mv.size());
    for x in mv.values() {
        match x {
            Value::Number(n) => v.push(*n),
            Value::Error(e) => return Err(e.clone()),
            _ => return Err(e_valor()),
        }
    }
    let mut d = Vec::with_capacity(md.size());
    for x in md.values() {
        match x {
            Value::Number(n) => d.push(n.trunc()),
            Value::Error(e) => return Err(e.clone()),
            Value::Text(_) => d.push(num(x).map_err(|_| e_valor())?.trunc()),
            _ => return Err(e_valor()),
        }
    }
    if d.iter().any(|x| *x < 0.0 || *x < d[0]) {
        return Err(e_num());
    }
    Ok((v, d))
}

/// Valor presente de fluxos datados e a derivada dele em relação à taxa.
fn xvpl(taxa: f64, v: &[f64], d: &[f64]) -> (f64, f64) {
    let base = 1.0 + taxa;
    let (mut y, mut dy) = (0.0, 0.0);
    for (x, dia) in v.iter().zip(d) {
        let t = (dia - d[0]) / 365.0;
        let fator = base.powf(t);
        y += x / fator;
        dy -= t * x / (fator * base);
    }
    (y, dy)
}

/// `=XVPL(taxa; valores; datas)` / `=XNPV(rate; values; dates)` — valor presente de fluxos em datas quaisquer.
pub fn fn_xnpv(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    if let Some(e) = arity(args, 3, 3) {
        return Computed::Scalar(Value::Error(e));
    }
    concluir((|| {
        let taxa = ev.number(&args[0])?;
        let (v, d) = fluxos_datados(ev, &args[1], &args[2])?;
        if taxa <= -1.0 {
            return Err(e_num());
        }
        numero(xvpl(taxa, &v, &d).0)
    })())
}

/// Precisa de pelo menos um fluxo positivo e um negativo para ter taxa interna.
fn tem_sinais_trocados(v: &[f64]) -> bool {
    v.iter().any(|x| *x > 0.0) && v.iter().any(|x| *x < 0.0)
}

/// `=XTIR(valores; datas; [estimativa])` / `=XIRR(values; dates; [guess])` — taxa interna de fluxos em datas quaisquer.
pub fn fn_xirr(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    if let Some(e) = arity(args, 2, 3) {
        return Computed::Scalar(Value::Error(e));
    }
    concluir((|| {
        let (v, d) = fluxos_datados(ev, &args[0], &args[1])?;
        let chute = match args.get(2) {
            None | Some(Node::Blank) => 0.1,
            Some(n) => ev.number(n)?,
        };
        if !tem_sinais_trocados(&v) {
            return Err(e_num());
        }
        match raiz_de_taxa(|r| xvpl(r, &v, &d), chute, 100, 1e-8) {
            Some(r) => numero(r),
            None => Err(e_num()),
        }
    })())
}

/// `=TIR(valores; [estimativa])` / `=IRR(values; [guess])` — taxa interna de retorno.
///
/// Newton a partir da estimativa (10%); se escapar, busca a raiz por
/// bisseção. #NÚM! quando os fluxos não têm taxa interna.
pub fn fn_irr(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    if let Some(e) = arity(args, 1, 2) {
        return Computed::Scalar(Value::Error(e));
    }
    concluir((|| {
        let v = numeros_de(ev, &args[0])?;
        let chute = match args.get(1) {
            None | Some(Node::Blank) => 0.1,
            Some(n) => ev.number(n)?,
        };
        if !tem_sinais_trocados(&v) {
            return Err(e_num());
        }
        let f = |r: f64| -> (f64, f64) {
            let base = 1.0 + r;
            let (mut y, mut dy) = (0.0, 0.0);
            for (i, x) in v.iter().enumerate() {
                let t = i as f64;
                let fator = base.powf(t);
                y += x / fator;
                dy -= t * x / (fator * base);
            }
            (y, dy)
        };
        match raiz_de_taxa(f, chute, 20, 1e-7) {
            Some(r) => numero(r),
            None => Err(e_num()),
        }
    })())
}

/// `=MTIR(valores; taxa_financ; taxa_reinvest)` / `=MIRR(values; finance_rate; reinvest_rate)` — taxa interna modificada.
pub fn fn_mirr(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    if let Some(e) = arity(args, 3, 3) {
        return Computed::Scalar(Value::Error(e));
    }
    concluir((|| {
        let v = numeros_de(ev, &args[0])?;
        let financ = ev.number(&args[1])?;
        let reinv = ev.number(&args[2])?;
        let n = v.len() as f64;
        let positivos: Vec<f64> = v.iter().map(|x| x.max(0.0)).collect();
        let negativos: Vec<f64> = v.iter().map(|x| x.min(0.0)).collect();
        let vf_pos = vpl(reinv, &positivos, 0.0) * (1.0 + reinv).powf(n - 1.0);
        let vp_neg = vpl(financ, &negativos, 0.0);
        if vf_pos == 0.0 || vp_neg == 0.0 || n < 2.0 {
            return Err(e_div0());
        }
        numero((-vf_pos / vp_neg).powf(1.0 / (n - 1.0)) - 1.0)
    })())
}

/// `=VFPLANO(capital; plano)` / `=FVSCHEDULE(principal; schedule)` — valor futuro com uma taxa diferente por período.
pub fn fn_fvschedule(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    if let Some(e) = arity(args, 2, 2) {
        return Computed::Scalar(Value::Error(e));
    }
    concluir((|| {
        let mut capital = ev.number(&args[0])?;
        let mut ruim = false;
        let erro = ev.for_each_value(&args[1], &mut |v, de_colecao| match v {
            Value::Number(r) => capital *= 1.0 + r,
            Value::Empty if de_colecao => {}
            _ => ruim = true,
        });
        if let Some(e) = erro {
            return Err(e);
        }
        if ruim {
            return Err(e_valor());
        }
        numero(capital)
    })())
}

#[cfg(test)]
mod testes {
    use crate::celulas::values::Value;
    use crate::planilha::sheet::Sheet;

    fn calc(formula: &str) -> Value {
        let mut s = Sheet::new("Plan1");
        s.set_cell_raw(0, 0, formula.into(), None);
        s.get_value(0, 0)
    }

    fn perto(formula: &str, esperado: f64, tol: f64) {
        match calc(formula) {
            Value::Number(n) => assert!((n - esperado).abs() <= tol, "{formula}: {n} ≠ {esperado}"),
            outro => panic!("{formula}: veio {outro:?}"),
        }
    }

    fn erro(formula: &str, codigo: &str) {
        match calc(formula) {
            Value::Error(e) => assert_eq!(e.code.as_str(), codigo, "{formula}"),
            outro => panic!("{formula}: esperava {codigo}, veio {outro:?}"),
        }
    }

    #[test]
    fn pagamentos_e_valores() {
        perto("=PGTO(0.08/12; 10; 10000)", -1037.032089, 1e-5);
        perto("=PGTO(0; 10; 1000)", -100.0, 1e-12);
        perto("=VF(0.06/12; 10; -200; -500; 1)", 2581.403374, 1e-5);
        perto("=VP(0.08/12; 12*20; 500; ; 0)", -59777.14585, 1e-4);
        perto("=NPER(0.12/12; -100; -1000; 10000; 1)", 59.67386567, 1e-7);
        perto("=IPGTO(0.1/12; 1; 3*12; 8000)", -66.66666667, 1e-7);
        perto("=IPGTO(0.1; 3; 3; 8000)", -292.4471299, 1e-6);
        perto("=PPGTO(0.1/12; 1; 24; 2000)", -75.62318601, 1e-7);
        perto("=ÉPGTO(0.1/12; 1; 36; 8000000)", -64814.81481, 1e-4);
        erro("=IPGTO(0.1; 0; 3; 8000)", "#NUM!");
    }

    #[test]
    fn depreciacao() {
        perto("=DPD(30000; 7500; 10)", 2250.0, 1e-9);
        perto("=SDA(30000; 7500; 10; 1)", 4090.909091, 1e-5);
        perto("=BD(1000000; 100000; 6; 1; 7)", 186083.3333, 1e-3);
        perto("=BD(1000000; 100000; 6; 7; 7)", 15845.10, 1e-2);
        perto("=BDD(2400; 300; 10*365; 1)", 1.315068493, 1e-8);
        perto("=BDD(2400; 300; 10; 10)", 22.1225472, 1e-6);
        perto("=BDV(2400; 300; 120; 6; 18)", 396.3060533, 1e-6);
        perto("=BDV(2400; 300; 10; 0; 0.875; 1.5)", 315.0, 1e-9);
        perto("=BDV(2400; 300; 120; 6; 18; 1.5)", 311.8089366, 1e-6);
    }

    #[test]
    fn taxas_e_cotacoes() {
        perto("=EFETIVA(0.0525; 4)", 0.053542667, 1e-8);
        perto("=NOMINAL(0.053543; 4)", 0.052500319, 1e-8);
        perto("=MOEDADEC(1.02; 16)", 1.125, 1e-12);
        perto("=MOEDAFRA(1.125; 16)", 1.02, 1e-12);
        erro("=MOEDADEC(1.02; 0)", "#DIV/0!");
        perto("=TAXAJURO(96; 10000; 11000)", 0.000993307, 1e-8);
        perto("=DURAÇÃOP(0.025; 2000; 2200)", 3.859866163, 1e-8);
        perto("=TAXA(4*12; -200; 8000)", 0.007701472, 1e-8);
    }

    #[test]
    fn fluxos_de_caixa() {
        perto("=VPL(0.1; -10000; 3000; 4200; 6800)", 1188.443412, 1e-5);
        perto("=TIR({-70000;12000;15000;18000;21000;26000})", 0.086630948, 1e-8);
        perto("=MTIR({-120000;39000;30000;21000;37000;46000}; 0.1; 0.12)", 0.126094, 1e-6);
        perto(
            "=XVPL(0.09; {-10000;2750;4250;3250;2750}; {39448;39508;39751;39859;39904})",
            2086.647602,
            1e-5,
        );
        perto("=XTIR({-10000;2750;4250;3250;2750}; {39448;39508;39751;39859;39904})", 0.373362535, 1e-8);
        perto("=PGTOJURACUM(0.09/12; 360; 125000; 13; 24; 0)", -11135.23213, 1e-4);
        perto("=PGTOCAPACUM(0.09/12; 360; 125000; 13; 24; 0)", -934.1071234, 1e-6);
        perto("=VFPLANO(1; {0.09;0.11;0.1})", 1.33089, 1e-12);
        erro("=TIR({1;2;3})", "#NUM!");
    }
}
