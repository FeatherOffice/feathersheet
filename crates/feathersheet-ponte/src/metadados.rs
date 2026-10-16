//! Os metadados da aba que andam com a estrutura, no JSON que a interface
//! guarda (`io/metadados-aba.ts`: `w`, `h`, `mc`, `tb`, `dn`, `pr`, `r`).
//!
//! A interface manda estes campos junto do `estrutura_mover` e recebe de
//! volta já acertados: as contas são do `feathersheet_core::operacoes::reflow`, e
//! aqui só se anda pelo JSON. Campo que a conta não conhece (o `sheet` de um
//! retângulo, o estilo de uma tabela) passa intacto, como o `{ ...rect }` do
//! TypeScript fazia.

use serde_json::{json, Map, Value as Json};
use feathersheet_core::formula::formula_text::{adjust_for_structure, Axis, StructureAdjustOptions};
use feathersheet_core::operacoes::reflow::{
    ajustar_intervalo, ajustar_intervalo_de_regra, ajustar_intervalo_tabela, ajustar_tamanhos,
};

pub struct Movimento<'a> {
    pub coluna: bool,
    pub em: u32,
    pub quantas: u32,
    pub removendo: bool,
    pub aba: &'a str,
}

impl Movimento<'_> {
    /// O deslocamento com sinal, como o `ajustarRect` recebe.
    fn delta(&self) -> i64 {
        if self.removendo {
            -(self.quantas as i64)
        } else {
            self.quantas as i64
        }
    }

    fn chaves(&self) -> (&'static str, &'static str) {
        if self.coluna {
            ("c0", "c1")
        } else {
            ("r0", "r1")
        }
    }
}

fn numero(o: &Map<String, Json>, k: &str) -> Option<i64> {
    o.get(k).and_then(Json::as_f64).map(|n| n as i64)
}

/// O retângulo `rect` depois do movimento (`ajustarRect`), ou `None`.
fn ajustar_rect(rect: &Json, m: &Movimento) -> Option<Json> {
    ajustar_rect_interno(rect, m, false)
}

fn ajustar_rect_tabela(rect: &Json, m: &Movimento) -> Option<Json> {
    ajustar_rect_interno(rect, m, true)
}

fn ajustar_rect_interno(rect: &Json, m: &Movimento, esticar_borda: bool) -> Option<Json> {
    let o = rect.as_object()?;
    let (ka, kb) = m.chaves();
    let (a, b) = if esticar_borda {
        ajustar_intervalo_tabela(numero(o, ka)?, numero(o, kb)?, m.em as i64, m.delta())?
    } else {
        ajustar_intervalo(numero(o, ka)?, numero(o, kb)?, m.em as i64, m.delta())?
    };
    let mut novo = o.clone();
    novo.insert(ka.into(), json!(a));
    novo.insert(kb.into(), json!(b));
    Some(Json::Object(novo))
}

/// Uma mescla de uma célula só não é mescla (`vale` do `mesclas.ts`).
fn mescla_vale(rect: &Json) -> bool {
    let n = |k| rect.get(k).and_then(Json::as_f64).unwrap_or(0.0);
    n("r1") > n("r0") || n("c1") > n("c0")
}

fn lista<'a>(meta: &'a mut Json, k: &str) -> Option<&'a mut Vec<Json>> {
    meta.get_mut(k).and_then(Json::as_array_mut)
}

/// Acerta, no lugar, os campos de `meta` que andam com a estrutura.
pub fn mover(meta: &mut Json, m: &Movimento) {
    // Larguras (coluna) ou alturas (linha).
    let campo = if m.coluna { "w" } else { "h" };
    if let Some(pares) = lista(meta, campo) {
        let tamanhos: Vec<(u32, u32)> = pares
            .iter()
            .filter_map(|p| Some((p.get(0)?.as_f64()? as u32, p.get(1)?.as_f64()? as u32)))
            .collect();
        *pares = ajustar_tamanhos(&tamanhos, m.em, m.quantas, m.removendo)
            .into_iter()
            .map(|(i, t)| json!([i, t]))
            .collect();
    }

    // Mesclas, tabelas e proteções: o retângulo anda, e o que some sai.
    if let Some(mesclas) = lista(meta, "mc") {
        *mesclas = mesclas.iter().filter_map(|r| ajustar_rect(r, m)).filter(mescla_vale).collect();
    }
    if let Some(tabelas) = lista(meta, "tb") {
        *tabelas = tabelas
            .iter()
            .filter_map(|t| {
                let so_filtro = t.get("soFiltro").and_then(Json::as_bool).unwrap_or(false);
                let area = if so_filtro {
                    ajustar_rect(t.get("area")?, m)?
                } else {
                    ajustar_rect_tabela(t.get("area")?, m)?
                };
                let mut t = t.clone();
                t["area"] = area;
                Some(t)
            })
            .collect();
    }
    if let Some(protecoes) = lista(meta, "pr") {
        *protecoes = protecoes
            .iter()
            .filter_map(|p| {
                let area = ajustar_rect(p.get("area")?, m)?;
                // Só os quatro números, como o `reflowProtecoes`.
                let so_numeros = json!({ "r0": area["r0"], "c0": area["c0"], "r1": area["r1"], "c1": area["c1"] });
                let mut p = p.clone();
                p["area"] = so_numeros;
                Some(p)
            })
            .collect();
    }

    // Dinâmicas: a fonte decide se a dinâmica fica; o destino é um canto.
    if let Some(dinamicas) = lista(meta, "dn") {
        *dinamicas = dinamicas
            .iter()
            .filter_map(|d| {
                let area = ajustar_rect_tabela(d.get("fonte")?.get("area")?, m)?;
                let mut d = d.clone();
                d["fonte"]["area"] = area;
                if let Some(resultado) = d.get("areaResultado").filter(|r| r.is_object()).cloned() {
                    d["areaResultado"] = ajustar_rect(&resultado, m).unwrap_or(Json::Null);
                }
                if let Some(destino) = d.get("destino").cloned() {
                    let (row, col) = (destino.get("row").cloned(), destino.get("col").cloned());
                    let canto = json!({ "r0": row, "c0": col, "r1": row, "c1": col });
                    if let Some(c) = ajustar_rect(&canto, m) {
                        d["destino"] = json!({ "row": c["r0"], "col": c["c0"] });
                    }
                }
                Some(d)
            })
            .collect();
    }

    // Regras: cada intervalo anda (conta própria do `reflowRules`), e a
    // fórmula da regra personalizada tem as referências consertadas.
    if let Some(regras) = lista(meta, "r") {
        *regras = regras.iter().filter_map(|regra| mover_regra(regra, m)).collect();
    }

    // Validações de célula: cada intervalo anda como o de uma regra; a
    // origem de uma validação por intervalo (lista vinda de células) anda
    // junto, sem apagar a validação se a origem não sobreviver ao todo.
    if let Some(validacoes) = lista(meta, "va") {
        *validacoes = validacoes.iter().filter_map(|v| mover_validacao(v, m)).collect();
    }
}

fn mover_regra(regra: &Json, m: &Movimento) -> Option<Json> {
    let (ka, kb) = m.chaves();
    let intervalos: Vec<Json> = regra
        .get("ranges")?
        .as_array()?
        .iter()
        .filter_map(|r| {
            let o = r.as_object()?;
            let (a, b) = ajustar_intervalo_de_regra(numero(o, ka)?, numero(o, kb)?, m.em as i64, m.quantas as i64, m.removendo)?;
            let mut novo = o.clone();
            novo.insert(ka.into(), json!(a));
            novo.insert(kb.into(), json!(b));
            Some(Json::Object(novo))
        })
        .collect();
    if intervalos.is_empty() {
        return None;
    }
    let mut regra = regra.clone();
    regra["ranges"] = Json::Array(intervalos);
    let cond = &regra["condition"];
    let personalizada = cond.get("kind").and_then(Json::as_str) == Some("cell")
        && cond.get("op").and_then(Json::as_str) == Some("formula");
    if let Some(cru) = personalizada.then(|| cond["values"].get(0).and_then(Json::as_str)).flatten().filter(|s| !s.is_empty()) {
        let texto = if cru.starts_with('=') { cru.to_string() } else { format!("={cru}") };
        let movida = adjust_for_structure(
            &texto,
            StructureAdjustOptions {
                axis: if m.coluna { Axis::Col } else { Axis::Row },
                at: m.em as i32,
                count: m.quantas as i32,
                removing: m.removendo,
                sheet: m.aba,
                own_sheet: m.aba,
            },
        );
        if movida != texto {
            regra["condition"]["values"][0] = json!(movida);
        }
    }
    Some(regra)
}

/// Como `mover_regra`, mas para uma validação de célula (`Validacao` do
/// `sheet-core::operacoes::validation`, no formato de `core/validation.ts`):
/// `{ ranges, regra: { tipo, ... }, quando, mensagem }`. Some quando os
/// intervalos da própria validação não sobrevivem; a origem de uma regra
/// `intervalo` (lista vinda de células) anda junto quando dá, e fica como
/// estava quando não — perder a validação inteira por causa da origem seria
/// pior que uma lista que não encolheu.
fn mover_validacao(v: &Json, m: &Movimento) -> Option<Json> {
    let (ka, kb) = m.chaves();
    let intervalos: Vec<Json> = v
        .get("ranges")?
        .as_array()?
        .iter()
        .filter_map(|r| {
            let o = r.as_object()?;
            let (a, b) = ajustar_intervalo_de_regra(numero(o, ka)?, numero(o, kb)?, m.em as i64, m.quantas as i64, m.removendo)?;
            let mut novo = o.clone();
            novo.insert(ka.into(), json!(a));
            novo.insert(kb.into(), json!(b));
            Some(Json::Object(novo))
        })
        .collect();
    if intervalos.is_empty() {
        return None;
    }
    let mut v = v.clone();
    v["ranges"] = Json::Array(intervalos);
    if v["regra"].get("tipo").and_then(Json::as_str) == Some("intervalo") {
        if let Some(origem) = v["regra"]["origem"].as_object().cloned() {
            if let (Some(a0), Some(b0)) = (numero(&origem, ka), numero(&origem, kb)) {
                if let Some((a, b)) = ajustar_intervalo_de_regra(a0, b0, m.em as i64, m.quantas as i64, m.removendo) {
                    let mut nova = origem;
                    nova.insert(ka.into(), json!(a));
                    nova.insert(kb.into(), json!(b));
                    v["regra"]["origem"] = Json::Object(nova);
                }
            }
        }
    }
    Some(v)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn linha(em: u32, quantas: u32, removendo: bool) -> Movimento<'static> {
        Movimento { coluna: false, em, quantas, removendo, aba: "Planilha1" }
    }

    #[test]
    fn inserir_linha_empurra_tabela_protecao_mescla_e_altura() {
        let mut meta = json!({
            "h": [[1, 30], [6, 40]],
            "w": [[2, 99]],
            "tb": [{ "id": "t", "area": { "r0": 5, "c0": 0, "r1": 9, "c1": 3 }, "estilo": { "tema": "azul" } }],
            "pr": [{ "area": { "r0": 5, "c0": 0, "r1": 5, "c1": 0, "sheet": "x" }, "hash": "h" }],
            "mc": [{ "r0": 7, "c0": 0, "r1": 8, "c1": 0 }],
        });
        mover(&mut meta, &linha(2, 3, false));
        assert_eq!(meta["h"], json!([[1, 30], [9, 40]]));
        assert_eq!(meta["w"], json!([[2, 99]]), "a largura não anda com linha");
        assert_eq!(meta["tb"][0]["area"], json!({ "r0": 8, "c0": 0, "r1": 12, "c1": 3 }));
        assert_eq!(meta["tb"][0]["estilo"]["tema"], "azul");
        assert_eq!(meta["pr"][0]["area"], json!({ "r0": 8, "c0": 0, "r1": 8, "c1": 0 }));
        assert_eq!(meta["mc"][0], json!({ "r0": 10, "c0": 0, "r1": 11, "c1": 0 }));
    }

    #[test]
    fn excluir_apaga_o_que_some_e_encolhe_a_mescla() {
        let mut meta = json!({
            "tb": [{ "area": { "r0": 5, "c0": 0, "r1": 9, "c1": 3 } }],
            "mc": [{ "r0": 5, "c0": 0, "r1": 6, "c1": 0 }],
            "dn": [{ "fonte": { "area": { "r0": 0, "c0": 0, "r1": 20, "c1": 2 } }, "destino": { "row": 30, "col": 5 }, "areaResultado": null }],
        });
        mover(&mut meta, &linha(5, 5, true));
        assert_eq!(meta["tb"], json!([]));
        assert_eq!(meta["mc"], json!([]), "mescla de uma célula só some");
        assert_eq!(meta["dn"][0]["fonte"]["area"]["r1"], 15);
        assert_eq!(meta["dn"][0]["destino"], json!({ "row": 25, "col": 5 }));
        assert_eq!(meta["dn"][0]["areaResultado"], Json::Null);
    }

    #[test]
    fn a_regra_anda_e_a_formula_dela_tambem() {
        let mut meta = json!({ "r": [
            { "id": "a", "ranges": [{ "r0": 5, "c0": 0, "r1": 9, "c1": 0 }],
              "condition": { "kind": "cell", "op": "formula", "values": ["=A6>1"] } },
            { "id": "b", "ranges": [{ "r0": 2, "c0": 0, "r1": 2, "c1": 0 }],
              "condition": { "kind": "cell", "op": "greaterThan", "values": ["1"] } },
        ]});
        mover(&mut meta, &linha(0, 2, false));
        assert_eq!(meta["r"][0]["ranges"][0], json!({ "r0": 7, "c0": 0, "r1": 11, "c1": 0 }));
        assert_eq!(meta["r"][0]["condition"]["values"][0], "=A8>1");
        assert_eq!(meta["r"][1]["ranges"], json!([{ "r0": 4, "c0": 0, "r1": 4, "c1": 0 }]));
        // Excluir abaixo não mexe nela; excluir dentro da outra encolhe.
        mover(&mut meta, &linha(8, 1, true));
        assert_eq!(meta["r"][1]["ranges"], json!([{ "r0": 4, "c0": 0, "r1": 4, "c1": 0 }]));
        assert_eq!(meta["r"][0]["ranges"][0], json!({ "r0": 7, "c0": 0, "r1": 10, "c1": 0 }));
        let mut so_uma = json!({ "r": [{ "ranges": [{ "r0": 4, "c0": 0, "r1": 4, "c1": 0 }], "condition": {} }] });
        mover(&mut so_uma, &linha(4, 1, true));
        assert_eq!(so_uma["r"], json!([]), "sem intervalo, a regra sai");
    }

    #[test]
    fn a_validacao_anda_e_a_origem_da_regra_de_intervalo_tambem() {
        let mut meta = json!({ "va": [
            { "ranges": [{ "r0": 5, "c0": 0, "r1": 9, "c1": 0 }],
              "regra": { "tipo": "intervalo", "origem": { "r0": 20, "c0": 3, "r1": 25, "c1": 3 } },
              "quando": "recusar" },
            { "ranges": [{ "r0": 2, "c0": 0, "r1": 2, "c1": 0 }],
              "regra": { "tipo": "numero", "op": "maior", "a": 0.0 },
              "quando": "avisar" },
        ]});
        mover(&mut meta, &linha(0, 2, false));
        assert_eq!(meta["va"][0]["ranges"][0], json!({ "r0": 7, "c0": 0, "r1": 11, "c1": 0 }));
        assert_eq!(meta["va"][0]["regra"]["origem"], json!({ "r0": 22, "c0": 3, "r1": 27, "c1": 3 }));
        assert_eq!(meta["va"][1]["ranges"], json!([{ "r0": 4, "c0": 0, "r1": 4, "c1": 0 }]));
        // A regra numérica não tem origem: nada quebra por faltar o campo.
        assert_eq!(meta["va"][1]["regra"]["op"], "maior");

        let mut so_uma = json!({ "va": [{ "ranges": [{ "r0": 4, "c0": 0, "r1": 4, "c1": 0 }],
            "regra": { "tipo": "caixa", "marcado": "sim", "desmarcado": "não" }, "quando": "avisar" }] });
        mover(&mut so_uma, &linha(4, 1, true));
        assert_eq!(so_uma["va"], json!([]), "sem intervalo, a validação sai");
    }
}
