//! "Atingir meta" do Excel: acha o valor de entrada que faz uma fórmula
//! chegar a um alvo. A busca é numérica e não conhece a planilha: quem
//! chama passa `f(x)`, que escreve `x` na célula variável e devolve o
//! resultado da fórmula (ou `None` quando ela dá erro).
//!
//! Método: secante a partir do valor atual; quando aparece troca de sinal,
//! guarda o intervalo e cai para bissecção se a secante sair dele. Mesmos
//! padrões do Excel: 100 iterações, tolerância 0,001 no resultado.

/// Padrões do Excel (Arquivo → Opções → Fórmulas → Iteração).
pub const ITERACOES: usize = 100;
pub const TOLERANCIA: f64 = 0.001;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Busca {
    /// Melhor entrada encontrada e o resultado da fórmula com ela.
    pub x: f64,
    pub y: f64,
    /// `true` quando |y − alvo| ficou dentro da tolerância.
    pub convergiu: bool,
    pub tentativas: usize,
}

/// Procura `x` com `f(x) ≈ alvo`, começando em `inicio`.
/// Devolve `None` se a fórmula der erro já no ponto de partida.
pub fn buscar(mut f: impl FnMut(f64) -> Option<f64>, inicio: f64, alvo: f64,
    iteracoes: usize, tolerancia: f64) -> Option<Busca> {
    let mut g = |x: f64| f(x).filter(|y| y.is_finite()).map(|y| (y, y - alvo));
    let (y0, mut d0) = g(inicio)?;
    let mut x0 = inicio;
    let mut melhor = Busca { x: x0, y: y0, convergiu: d0.abs() <= tolerancia, tentativas: 1 };
    if melhor.convergiu { return Some(melhor); }
    // Segundo ponto: um passo proporcional ao valor (ou 0,01 perto de zero).
    let mut x1 = if x0 == 0.0 { 0.01 } else { x0 * 1.01 };
    // Intervalo com troca de sinal, quando houver: (x com d<0, x com d>0).
    let mut cerco: Option<(f64, f64)> = None;
    for tentativa in 2..=iteracoes {
        let Some((y1, d1)) = g(x1) else {
            // Erro (ex.: divisão por zero): recua até a metade do passo.
            x1 = (x0 + x1) / 2.0;
            continue;
        };
        if d1.abs() < (melhor.y - alvo).abs() {
            melhor = Busca { x: x1, y: y1, convergiu: d1.abs() <= tolerancia, tentativas: tentativa };
            if melhor.convergiu { return Some(melhor); }
        }
        melhor.tentativas = tentativa;
        if d0.signum() != d1.signum() {
            cerco = Some(if d0 < 0.0 { (x0, x1) } else { (x1, x0) });
        } else if let Some((neg, pos)) = cerco {
            // Mantém o cerco atualizado com o ponto novo.
            cerco = Some(if d1 < 0.0 { (x1, pos) } else { (neg, x1) });
        }
        let mut proximo = if d1 != d0 { x1 - d1 * (x1 - x0) / (d1 - d0) } else { x1 * 2.0 + 1.0 };
        if let Some((neg, pos)) = cerco {
            let (lo, hi) = (neg.min(pos), neg.max(pos));
            if !(proximo > lo && proximo < hi) || !proximo.is_finite() { proximo = (neg + pos) / 2.0; }
        }
        if !proximo.is_finite() { break; }
        x0 = x1;
        d0 = d1;
        x1 = proximo;
    }
    Some(melhor)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn linear_acha_na_hora_e_quadratica_converge() {
        // B10 = B3 * 2 + 10, quer 1000 -> B3 = 495.
        let r = buscar(|x| Some(x * 2.0 + 10.0), 1.0, 1000.0, ITERACOES, TOLERANCIA).unwrap();
        assert!(r.convergiu && (r.x - 495.0).abs() < 0.01, "{r:?}");
        let r = buscar(|x| Some(x * x), 3.0, 2.0, ITERACOES, TOLERANCIA).unwrap();
        assert!(r.convergiu && (r.x.abs() - 2f64.sqrt()).abs() < 0.001, "{r:?}");
        // Partindo de zero também anda.
        let r = buscar(|x| Some(x * 3.0), 0.0, 9.0, ITERACOES, TOLERANCIA).unwrap();
        assert!(r.convergiu && (r.x - 3.0).abs() < 0.001, "{r:?}");
    }

    #[test]
    fn sem_solucao_devolve_o_mais_perto_e_erro_na_partida_e_none() {
        // x² nunca vale -5: a melhor tentativa fica perto de 0.
        let r = buscar(|x| Some(x * x), 2.0, -5.0, ITERACOES, TOLERANCIA).unwrap();
        assert!(!r.convergiu && r.y < 1.0, "{r:?}");
        assert!(buscar(|_| None, 1.0, 5.0, ITERACOES, TOLERANCIA).is_none());
    }

    #[test]
    fn erro_no_caminho_recua_o_passo() {
        // 1/(x-1) dá erro em 1; quer 0,5 -> x = 3.
        let f = |x: f64| (x != 1.0).then(|| 1.0 / (x - 1.0));
        let r = buscar(f, 2.0, 0.5, ITERACOES, TOLERANCIA).unwrap();
        assert!(r.convergiu && (r.x - 3.0).abs() < 0.01, "{r:?}");
    }
}
