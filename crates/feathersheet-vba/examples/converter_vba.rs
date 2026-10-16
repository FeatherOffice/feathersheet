//! `cargo run --example converter_vba -- arquivo.xlsm [pasta-de-saída]`: converte as macros
//! VBA do arquivo e mostra o resumo; com a pasta, grava cada macro e evento como `.js` nela
//! (e `pendencias.txt`).

use std::collections::BTreeMap;

fn main() {
    let mut args = std::env::args().skip(1);
    let caminho = args.next().expect("passe o caminho do .xlsm");
    let saida = args.next();
    let bytes = std::fs::read(&caminho).expect("não consegui ler o arquivo");
    let nome = std::path::Path::new(&caminho).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let inicio = std::time::Instant::now();
    let c = feathersheet_vba::converter_xlsm(&bytes, &nome).unwrap_or_else(|e| panic!("{e}"));
    let ms = inicio.elapsed().as_secs_f64() * 1000.0;
    let pendentes = c.pendencias.iter().filter(|p| !p.aviso).count();
    println!("[vba] {} linhas de VBA → {} macros, {} eventos em {ms:.1} ms", c.linhas_vba, c.macros.len(), c.eventos.len());
    println!("      {} instruções convertidas, {} trechos não convertidos, {} avisos", c.instrucoes_convertidas, pendentes, c.pendencias.len() - pendentes);
    let mut por_motivo: BTreeMap<&str, usize> = BTreeMap::new();
    for p in c.pendencias.iter().filter(|p| !p.aviso) {
        *por_motivo.entry(&p.motivo).or_default() += 1;
    }
    let mut v: Vec<_> = por_motivo.into_iter().collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    for (m, n) in v {
        println!("      {n:>4} × {m}");
    }
    for b in &c.botoes {
        println!("      botão \"{}\" → {}", b.forma, b.macro_);
    }
    let Some(pasta) = saida else { return };
    std::fs::create_dir_all(&pasta).unwrap();
    for m in &c.macros {
        std::fs::write(format!("{pasta}/{}.js", m.nome.replace(['/', ' '], "_")), &m.codigo).unwrap();
    }
    for e in &c.eventos {
        std::fs::write(format!("{pasta}/evento_{}_{}.js", e.aba.as_deref().unwrap_or("pasta").replace(['/', ' '], "_"), e.evento), &e.codigo).unwrap();
    }
    let lista: Vec<String> = c.pendencias.iter().map(|p| format!("{}{}:{} — {}\n    {}", if p.aviso { "[aviso] " } else { "" }, p.modulo, p.linha, p.motivo, p.vba.replace('\n', "\n    "))).collect();
    std::fs::write(format!("{pasta}/pendencias.txt"), lista.join("\n")).unwrap();
}
