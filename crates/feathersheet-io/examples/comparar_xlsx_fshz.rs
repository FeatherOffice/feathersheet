//! Lê o mesmo conteúdo de um `.xlsx` e de um `.fshz` e compara o texto de
//! todas as células de todas as abas: `comparar_xlsx_fshz A.xlsx B.fshz`.
fn main() {
    let xlsx = std::path::PathBuf::from(std::env::args().nth(1).expect("xlsx"));
    let fshz = std::path::PathBuf::from(std::env::args().nth(2).expect("fshz"));
    let a = feathersheet_io::xlsx_read::ler_xlsx_arquivo(&xlsx).expect("ler xlsx").0;
    let lida = feathersheet_io::fshz_read::ler_fshz_arquivo(&fshz).expect("ler fshz");
    let b = feathersheet_io::para_workbook::para_workbook(&lida);
    let mut diferentes = 0u64;
    let mut celulas = 0u64;
    for i in 0..a.len() {
        let sa = &a.aba(i).unwrap().sheet;
        let j = b.indice_por_nome(&sa.name).expect("aba no fshz");
        let sb = &b.aba(j).unwrap().sheet;
        for r in 0..=sa.max_row.max(sb.max_row) {
            for c in 0..=sa.max_col.max(sb.max_col) {
                celulas += 1;
                let (x, y) = (sa.get_raw(r, c), sb.get_raw(r, c));
                if x != y {
                    diferentes += 1;
                    if diferentes <= 10 { println!("{}!({r},{c}): xlsx {x:?} · fshz {y:?}", sa.name); }
                }
            }
        }
    }
    println!("{diferentes} diferença(s) em {celulas} células");
}
