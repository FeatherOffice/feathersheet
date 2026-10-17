//! Confere que dois `.fshz` têm as mesmas células nas colunas: mesmo número
//! de linhas, mesmos números, textos e fórmulas, coluna a coluna.
//!
//! ```bash
//! cargo run --release --example conferir_fshz_iguais -- a.fshz b.fshz
//! ```

fn main() {
    let mut args = std::env::args().skip(1);
    let a = args.next().expect("primeiro .fshz");
    let b = args.next().expect("segundo .fshz");
    let ler = |p: &str| feathersheet_io::fshz_read::ler_fshz_arquivo(std::path::Path::new(p)).expect("não abri");
    let (la, lb) = (ler(&a), ler(&b));
    assert_eq!(la.sheets.len(), lb.sheets.len(), "número de abas");
    for (x, y) in la.sheets.iter().zip(&lb.sheets) {
        assert_eq!(x.n, y.n);
        assert_eq!(x.avulsas.len(), y.avulsas.len(), "avulsas de {}", x.n);
        assert_eq!(x.colunas_num.len(), y.colunas_num.len());
        for (p, q) in x.colunas_num.iter().zip(&y.colunas_num) {
            assert_eq!(p.col, q.col);
            assert_eq!(p.bitmap, q.bitmap, "máscara da coluna {}", p.col);
            assert!(p.values.iter().zip(&q.values).enumerate().all(|(r, (u, v))|
                p.bitmap[r >> 3] & (1 << (r & 7)) == 0 || u.to_bits() == v.to_bits()), "números da coluna {}", p.col);
        }
        assert_eq!(x.colunas_txt.len(), y.colunas_txt.len());
        for (p, q) in x.colunas_txt.iter().zip(&y.colunas_txt) {
            assert_eq!(p.col, q.col);
            assert_eq!(p.bitmap, q.bitmap, "máscara da coluna {}", p.col);
            assert!(p.indices.iter().zip(&q.indices).enumerate().all(|(r, (u, v))|
                p.bitmap[r >> 3] & (1 << (r & 7)) == 0 || p.dict[*u as usize] == q.dict[*v as usize]), "textos da coluna {}", p.col);
        }
        assert_eq!(x.colunas_formula.len(), y.colunas_formula.len());
        println!("aba {}: {} num, {} texto, {} fórmula — iguais", x.n, x.colunas_num.len(), x.colunas_txt.len(), x.colunas_formula.len());
    }
}
