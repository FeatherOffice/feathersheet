//! Faz o que a janela da plan_a faz: abre o arquivo, calcula a célula com
//! vínculo e chama o `vinculos_atualizar` com a pasta dela.
//! `cargo run --example simular_vinculo -- plan_a.plz linha coluna`.
use serde_json::json;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(&a[0]).expect("ler");
    let (linha, coluna): (u32, u32) = (a[1].parse().unwrap(), a[2].parse().unwrap());
    let base = std::path::Path::new(&a[0]).parent().unwrap().to_string_lossy().into_owned();
    let mut p = feathersheet_ponte::Ponte::default();
    p.abrir_fshz(&bytes).unwrap();
    println!("antes: {:?}", p.pasta.valor(0, linha, coluna));
    let r = p.chamar("vinculos_atualizar", &serde_json::to_vec(&json!({ "base": base })).unwrap()).unwrap();
    let r: serde_json::Value = serde_json::from_slice(&r).unwrap();
    println!("vinculos_atualizar: mudou = {}", r["mudou"]);
    println!("depois: {:?}", p.pasta.valor(0, linha, coluna));
}
