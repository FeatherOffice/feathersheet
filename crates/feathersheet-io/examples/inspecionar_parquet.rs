//! Mostra esquema, linhas, grupos e compressão de um `.parquet`.
//!   cargo run --release --example inspecionar_parquet -- arquivo.parquet
use parquet::file::reader::{FileReader, SerializedFileReader};

fn main() -> Result<(), String> {
    let caminho = std::env::args().nth(1).ok_or("informe o arquivo")?;
    let f = std::fs::File::open(&caminho).map_err(|e| e.to_string())?;
    let r = SerializedFileReader::new(f).map_err(|e| e.to_string())?;
    let m = r.metadata();
    let fm = m.file_metadata();
    println!("{} linhas · {} grupos · criado por {:?}", fm.num_rows(), m.num_row_groups(), fm.created_by());
    let mut codecs = std::collections::BTreeSet::new();
    for g in m.row_groups() {
        for c in g.columns() {
            codecs.insert(format!("{:?}", c.compression()));
        }
    }
    println!("compressão: {codecs:?}");
    for c in fm.schema_descr().columns() {
        println!("  {} : {:?} / {:?}", c.name(), c.physical_type(), c.logical_type());
    }
    Ok(())
}
