//! Bancada reproduzível dos passos do plano de memória/velocidade.
//! `medir_plano_unico ARQUIVO [--salvar DESTINO] [--completo]`
//! Tempos de leitura de células são diagnóstico do motor, não quadros Iced.
use feathersheet_core::planilha::workbook::Workbook;
use std::{hint::black_box, path::Path, time::Instant};

fn ram() -> (f64, f64) {
    let s = std::fs::read_to_string("/proc/self/status").expect("RSS Linux");
    let campo = |n: &str| {
        s.lines()
            .find_map(|l| l.strip_prefix(n))
            .and_then(|l| l.split_whitespace().next()?.parse::<f64>().ok())
            .unwrap()
            / 1024.0
    };
    (campo("VmRSS:"), campo("VmHWM:"))
}

fn medida(nome: &str, inicio: Instant) {
    let (rss, pico) = ram();
    println!(
        "{nome}\t{:.6}\t{rss:.3}\t{pico:.3}",
        inicio.elapsed().as_secs_f64()
    );
}

fn abrir(p: &Path) -> Workbook {
    if p.extension().is_some_and(|e| e == "fshz") {
        let lida = feathersheet_io::fshz_read::ler_fshz_arquivo(p).expect("ler FSHZ");
        feathersheet_io::para_workbook::para_workbook(&lida)
    } else if arquivo_csv(p) {
        feathersheet_io::csv_workbook::importar_csv_tsv_arquivo(
            p,
            p.extension().is_some_and(|e| e == "tsv"),
            None,
        ).expect("ler CSV/TSV")
    } else {
        feathersheet_io::xlsx_read::ler_xlsx_arquivo(p)
            .expect("ler XLSX")
            .0
    }
}

fn arquivo_csv(p: &Path) -> bool {
    p.extension().is_some_and(|e| e == "csv" || e == "tsv")
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if !args.iter().any(|a| a == "--alocador-padrao") {
        feathersheet_core::paralelismo::ajustar_alocador();
    }
    let arquivo = Path::new(args.get(1).expect("informe ARQUIVO"));
    let nucleos = args.iter().position(|a| a == "--nucleos")
        .and_then(|i| args.get(i + 1)).and_then(|n| n.parse().ok()).unwrap_or(8);
    feathersheet_core::paralelismo::definir(nucleos);
    println!("medida\tsegundos\trss_mib\tpico_mib");
    let t = Instant::now();
    let mut w = abrir(arquivo);
    medida("abrir", t);
    if args.iter().any(|a| a == "--devolver") {
        let t = Instant::now();
        feathersheet_core::paralelismo::devolver_memoria_ao_sistema();
        medida("devolver_memoria", t);
    }
    if args.iter().any(|a| a == "--paralelo") {
        let t = Instant::now();
        let n = w.calcular_em_paralelo();
        medida("calcular_paralelo", t);
        println!("paralelo_celulas\t{n}\tthreads={}", feathersheet_core::paralelismo::nucleos());
        for linha in feathersheet_core::diario::recolher() {
            println!("diario\t{linha}");
        }
    }
    if let Some(dados) = w.indice_por_nome("Dados") {
        let fim = w.aba(dados).unwrap().sheet.used_rect().r1;
        let t = Instant::now();
        println!("corrente_valor\t{:?}", w.valor(dados, fim, 10));
        medida("corrente", t);
    }
    if let Some(resumo) = w.indice_por_nome("Resumo") {
        let usado = w.aba(resumo).unwrap().sheet.used_rect();
        let t = Instant::now();
        let mut soma = 0.0;
        for r in usado.r0..=usado.r1 {
            for c in usado.c0..=usado.c1 {
                if let feathersheet_core::Value::Number(n) = w.valor(resumo, r, c) {
                    soma += n;
                }
            }
        }
        medida("resumo", t);
        println!("resumo_soma\t{soma:.12}");
    }
    if args.iter().any(|a| a == "--completo") {
        let t = Instant::now();
        // Conferência: soma dos números e contagem de textos/erros/vazios de
        // todas as células, para comparar o resultado entre versões.
        let (mut soma, mut textos, mut erros, mut vazios) = (0.0f64, 0u64, 0u64, 0u64);
        for aba in 0..w.len() {
            let usado = w.aba(aba).unwrap().sheet.used_rect();
            for r in usado.r0..=usado.r1 {
                for c in usado.c0..=usado.c1 {
                    match black_box(w.valor(aba, r, c)) {
                        feathersheet_core::Value::Number(n) => soma += n,
                        feathersheet_core::Value::Text(_) => textos += 1,
                        feathersheet_core::Value::Error(_) => erros += 1,
                        feathersheet_core::Value::Empty => vazios += 1,
                        _ => {}
                    }
                }
            }
        }
        medida("calcular_tudo", t);
        println!("conferencia\tsoma={soma:.6}\ttextos={textos}\terros={erros}\tvazios={vazios}");
    }
    w.soltar_faixas();
    let t = Instant::now();
    let mut total = 0;
    loop {
        let n = w.consolidar_calculos(usize::MAX);
        if n == 0 {
            break;
        }
        total += n;
    }
    medida("calculada_consolidada", t);
    println!("consolidados\t{total}");
    // Grade de 40 × 14 células, em posições diferentes da aba Dados.
    // Mede só consultas e formatação; não inclui desenho/compositor/GPU.
    let aba = w.indice_por_nome("Dados").unwrap_or(0);
    let usado = w.aba(aba).unwrap().sheet.used_rect();
    let mut leituras = Vec::new();
    for i in 0..100 {
        let inicio = ((i as u64 * u64::from(usado.r1.saturating_sub(40))) / 99) as u32;
        let t = Instant::now();
        for r in inicio..=inicio.saturating_add(39).min(usado.r1) {
            for c in usado.c0..=usado.c1.min(13) {
                black_box(w.valor(aba, r, c));
                black_box(w.aba(aba).unwrap().sheet.get_style(r, c));
            }
        }
        leituras.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    leituras.sort_by(f64::total_cmp);
    println!(
        "consultas_grade_ms\tp50={:.6}\tp95={:.6}\tmax={:.6}",
        leituras[50], leituras[95], leituras[99]
    );
    if let Some(i) = args.iter().position(|a| a == "--salvar") {
        let destino = Path::new(args.get(i + 1).expect("informe DESTINO"));
        assert_ne!(
            arquivo.canonicalize().ok(),
            destino.canonicalize().ok().filter(|_| destino.exists()),
            "use um destino de medição diferente do original"
        );
        let t = Instant::now();
        feathersheet_io::da_workbook::pasta_gravar_arquivo(&w, 0, destino).expect("salvar cópia");
        medida("salvar", t);
    }
    for (i, nome) in w.nomes().iter().enumerate() {
        println!(
            "estrutura\t{nome}\t{}",
            w.aba(i).unwrap().sheet.contagem_memoria()
        );
    }
}
