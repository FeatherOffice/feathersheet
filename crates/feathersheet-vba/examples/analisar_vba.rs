//! `cargo run --example analisar_vba -- arquivo.xlsm`: o que o analisador não entendeu.

use feathersheet_vba::sintaxe::{analisar, Instr, TipoInstr};

fn desconhecidas<'a>(v: &'a [Instr], saida: &mut Vec<&'a Instr>) {
    for i in v {
        match &i.k {
            TipoInstr::Desconhecida(_) => saida.push(i),
            TipoInstr::Se { ramos, senao } => {
                ramos.iter().for_each(|(_, c)| desconhecidas(c, saida));
                if let Some(s) = senao {
                    desconhecidas(s, saida);
                }
            }
            TipoInstr::Escolha { casos, senao, .. } => {
                casos.iter().for_each(|(_, c)| desconhecidas(c, saida));
                if let Some(s) = senao {
                    desconhecidas(s, saida);
                }
            }
            TipoInstr::Para { corpo, .. } | TipoInstr::ParaCada { corpo, .. } | TipoInstr::Faca { corpo, .. } | TipoInstr::Com { corpo, .. } => desconhecidas(corpo, saida),
            _ => {}
        }
    }
}

fn main() {
    let caminho = std::env::args().nth(1).expect("passe o .xlsm");
    let p = feathersheet_vba::xlsm::ler(&std::fs::read(caminho).unwrap()).unwrap();
    let inicio = std::time::Instant::now();
    let (mut procs, mut ruins) = (0, 0);
    for m in &p.modulos {
        let a = analisar(&m.codigo);
        procs += a.procs.len();
        for (l, t) in &a.ignorados {
            println!("{} [módulo] {l}: {t}", m.nome);
        }
        let mut v = Vec::new();
        a.procs.iter().for_each(|p| desconhecidas(&p.corpo, &mut v));
        ruins += v.len();
        for i in v {
            if let TipoInstr::Desconhecida(t) = &i.k {
                println!("{} {}: {}", m.nome, i.linha, t.chars().take(110).collect::<String>());
            }
        }
    }
    println!("[analisar] {procs} procedimentos, {ruins} instruções desconhecidas, {:.1} ms", inicio.elapsed().as_secs_f64() * 1000.0);
}
