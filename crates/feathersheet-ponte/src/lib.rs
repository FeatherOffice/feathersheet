//! A ponte: o TypeScript manda um comando com argumentos em JSON, a pasta
//! viva no Rust responde.
//!
//! Um comando é um nome (`celula_gravar`, `bloco`, `desfazer`…) e um corpo
//! JSON; a resposta são bytes — JSON para quase tudo, binário para o que é
//! caminho quente (`bloco`, `celulas`, ver `feathersheet_core::planilha::bloco`). Quem entrega
//! os bytes de um lado para o outro não importa aqui, e isso é o ponto:
//!
//! - no aplicativo, o `src-tauri` atende o protocolo `pasta://` e chama
//!   [`Ponte::chamar`] — é uma chamada **síncrona** do lado do JavaScript
//!   (XHR síncrono), o que deixa a interface continuar lendo célula como
//!   sempre leu, sem `await` em 19 mil linhas;
//! - nos testes do TypeScript, este crate vira `.wasm` e o Node chama a mesma
//!   função (ver [`wasm`]). O teste roda contra o motor de verdade, não
//!   contra uma imitação dele.
//!
//! Todo comando que muda alguma coisa responde o [`Estado`] da pasta: a
//! versão, os limites e o desfazer de cada aba. O TypeScript usa a versão
//! para jogar fora os blocos velhos do cache.
//!
//! As abas são endereçadas pelo `id` (estável), nunca pelo índice.

use serde::Serialize;
use serde_json::{json, Value as Json};
use std::collections::{HashMap, HashSet};

use feathersheet_core::celulas::estilo_parcial::{para_style, Campo, EstiloParcial};
use feathersheet_core::operacoes::ops::{FaixasOcultas, FormatoDeData, TipoColunaDivisao};
use feathersheet_core::celulas::refs::Rect;
use feathersheet_core::planilha::sheet::edicao::Snap;
use feathersheet_core::planilha::sheet::Sheet;
use feathersheet_core::planilha::workbook::Workbook;

pub mod metadados;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

/// Uma pasta lida de um arquivo e já montada, pronta para entrar na ponte
/// ([`Ponte::instalar`]).
pub struct PastaAberta {
    pasta: Workbook,
    ativa: u32,
    nomes: Json,
}

impl PastaAberta {
    /// FSH3 lê o compressor diretamente do disco, sem buffer do arquivo inteiro.
    pub fn ler_arquivo(path: &std::path::Path) -> Result<Self, String> {
        let lida = feathersheet_io::fshz_read::ler_fshz_arquivo(path)?;
        let pasta = feathersheet_io::para_workbook::para_workbook(&lida);
        Ok(Self {
            nomes: feathersheet_io::para_workbook::nomes_para_json(&pasta),
            pasta,
            ativa: lida.a,
        })
    }

    /// Lê e monta a pasta — o trabalho pesado, que não precisa da ponte.
    pub fn ler(bytes: &[u8]) -> Result<Self, String> {
        let lida = feathersheet_io::fshz_read::ler_fshz(bytes)?;
        let pasta = feathersheet_io::para_workbook::para_workbook(&lida);
        Ok(PastaAberta {
            nomes: feathersheet_io::para_workbook::nomes_para_json(&pasta),
            pasta,
            ativa: lida.a,
        })
    }

    /// Monta a pasta a partir do formato do `Workbook.loadJSON` do
    /// TypeScript (`{ sheets: [{ name, cells: { chave: { raw, style } },
    /// colWidths, rowHeights, charts, rules }], names, active }`). É o que o
    /// leitor de `.xlsx` do `src-tauri` produz: montando aqui, as células não
    /// vão ao JavaScript para voltar ao Rust em lotes.
    pub fn de_json(livro: &Json) -> Self {
        let mut pasta = Workbook::vazia();
        for aba in livro.get("sheets").and_then(Json::as_array).into_iter().flatten() {
            let nome = aba.get("name").and_then(Json::as_str).unwrap_or("Planilha1");
            let i = pasta.criar_aba(nome);
            let s = &mut pasta.aba_mut(i).expect("acabou de ser criada").sheet;
            for (k, snap) in aba.get("cells").and_then(Json::as_object).into_iter().flatten() {
                let Ok(k) = k.parse::<u64>() else { continue };
                let (row, col) = Sheet::key_to_coord(k);
                s.apply(row, col, snap_de_json(snap), false);
            }
            let pares = |campo: &str| -> Vec<(u32, u32)> {
                aba.get(campo)
                    .and_then(Json::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|p| Some((p.get(0)?.as_f64()?.max(0.0) as u32, p.get(1)?.as_f64()?.max(0.0).round() as u32)))
                    .collect()
            };
            s.col_widths.extend(pares("colWidths"));
            s.row_heights.extend(pares("rowHeights"));
            let opaco = |campo: &str| aba.get(campo).filter(|v| v.as_array().is_some_and(|a| !a.is_empty())).map(Json::to_string);
            s.g = opaco("charts");
            s.r = opaco("rules");
            s.mc = opaco("mesclas");
            s.im = opaco("images");
            s.tb = opaco("tabelas");
            s.dn = opaco("dinamicas");
        }
        if pasta.is_empty() {
            pasta.criar_aba("Planilha1");
        }
        if let Some(nomes) = livro.get("names") {
            feathersheet_io::para_workbook::nomes_de_json(&mut pasta, nomes);
        }
        let ativa = livro.get("active").and_then(Json::as_u64).unwrap_or(0) as u32;
        PastaAberta { nomes: feathersheet_io::para_workbook::nomes_para_json(&pasta), pasta, ativa }
    }

    /// Devolve a pasta nativa, sem criar uma ponte web ou uma segunda cópia.
    /// Utilizado pelo frontend Iced ao importar arquivos Excel em Rust.
    pub fn extrair(self) -> (Workbook, u32) {
        (self.pasta, self.ativa)
    }
}

/// As regras de uma aba, lidas do JSON, e em que estado da pasta.
struct CondicionalGuardada {
    regras: String,
    versoes: u64,
    formatos: feathersheet_core::operacoes::rules::ConditionalFormats,
}

/// O que o motor de regras enxerga de uma aba da pasta viva.
struct HostDaRegra<'a> {
    pasta: &'a Workbook,
    i: usize,
    max_row: u32,
    max_col: u32,
}

impl feathersheet_core::operacoes::rules::RuleHost for HostDaRegra<'_> {
    fn valor(&self, row: u32, col: u32) -> feathersheet_core::Value {
        self.pasta.valor(self.i, row, col)
    }
    fn avaliar_em(&self, row: u32, col: u32, node: &feathersheet_core::Node) -> feathersheet_core::Value {
        self.pasta.avaliar_em(self.i, row, col, node)
    }
    fn max_row(&self) -> u32 {
        self.max_row
    }
    fn max_col(&self) -> u32 {
        self.max_col
    }
}

/// As regras do JSON da aba (`Sheet.r`, o `rules` do arquivo). Regra que não
/// lê (um operador novo que o Rust não conhece) fica de fora — as outras
/// continuam pintando.
fn regras_de_json(texto: &str) -> Vec<feathersheet_core::operacoes::rules::ConditionalRule> {
    serde_json::from_str::<Vec<Json>>(texto)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|r| serde_json::from_value(r).ok())
        .collect()
}

/// As validações do JSON da aba (`Sheet.va`, o `va` do arquivo). Mesmo
/// espírito do `regras_de_json`: entrada que não é uma `Validacao` válida
/// fica de fora, as outras continuam valendo.
fn validacoes_de_json(texto: &str) -> Vec<feathersheet_core::operacoes::validation::Validacao> {
    serde_json::from_str::<Vec<Json>>(texto)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect()
}

/// A pasta viva e quem a atende.
pub struct Ponte {
    pub pasta: Workbook,
    /// As linhas escondidas de cada aba (pelo `id`), como o TypeScript as
    /// mandou. As alturas ainda moram lá; isto é o que o rodapé precisa para
    /// não somar linha fora do filtro. Some quando a pasta é trocada: os
    /// `id`s recomeçam, e a faixa de uma aba velha cairia numa nova.
    ocultas: HashMap<u32, FaixasOcultas>,
    /// A formatação condicional de cada aba (pelo `id`), com as estatísticas
    /// das regras em cache. Vale enquanto o texto das regras e as versões das
    /// abas forem os mesmos: uma regra de fórmula pode ler outra aba.
    condicionais: HashMap<u32, CondicionalGuardada>,
    /// A última busca de cada aba (pelo `id`). A caixa de localizar pergunta
    /// de novo a cada "Próxima"; enquanto a aba não muda de versão e a
    /// consulta é a mesma, a lista já está pronta.
    buscas: HashMap<u32, BuscaGuardada>,
    /// A versão de cada aba (pelo `id`) que o último estado entregou ao
    /// TypeScript: é contra ela que se decide se a aba mudou e se as
    /// `tocadas` vão junto.
    versoes_informadas: HashMap<u32, u32>,
    /// Se o comando em curso está em [`COMANDOS_PRECISOS`]. Fora da lista, a
    /// aba que mudou vai com `tocadas: null` ("não sei onde").
    comando_preciso: bool,
    /// Os arquivos das pastas externas já carregados (caminho resolvido) e a
    /// data de modificação de quando foram lidos: mudou no disco, relê.
    vinculos: HashMap<String, Option<std::time::SystemTime>>,
    /// O caminho resolvido de cada pasta externa → o nome como a fórmula o
    /// escreve. A outra janela manda o caminho; o motor conhece o nome.
    nomes_dos_vinculos: HashMap<String, String>,
    /// As tabelas de origem do OpenQuery (pelo número que a janela deu):
    /// vão uma vez, e cada pré-visualização manda só os passos.
    consultas: HashMap<u32, feathersheet_core::Tabela>,
    /// O número da próxima tabela guardada. Quem numera é a ponte: as
    /// janelas do aplicativo dividem a mesma.
    proxima_consulta: u32,
}

/// Os comandos cujo rastro de células tocadas é confiável: tudo o que eles
/// escrevem passa pelo `set_cell_raw`/`apply` e pela cascata de "sujo" (ver
/// `sheet::Rastro`). Comando que não está aqui e muda uma aba a entrega como
/// "não sei onde", e o gráfico relê — lento, nunca errado. Para pôr um
/// comando aqui, conferir que ele não mexe em massa por fora desses caminhos
/// (ou que, quando mexe, declara o rastro desconhecido).
const COMANDOS_PRECISOS: &[&str] = &[
    "celula_gravar",
    "celula_aplicar",
    "celulas_aplicar",
    "limpar",
    "colar",
    "desfazer",
    "refazer",
    "mover_intervalo",
    "preencher",
    // Escreve pelo mesmo `Sheet::preencher` (set_cell + set_style).
    "preencher_ate_o_fim",
    "substituir",
    "estilo_aplicar",
];

struct BuscaGuardada {
    versao: u32,
    consulta: String,
    opcoes: feathersheet_core::SearchOptions,
    achadas: Vec<feathersheet_core::SearchMatch>,
}

impl Default for Ponte {
    fn default() -> Self {
        Ponte::com_pasta(Workbook::new())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EstadoAba {
    id: u32,
    nome: String,
    versao: u32,
    max_row: u32,
    max_col: u32,
    pode_desfazer: bool,
    pode_refazer: bool,
    /// Marcas do histórico (`profundidadeDoHistorico`/`…DoRefazer`): o
    /// desfazer de estrutura intercala com o de célula por elas.
    desfazer: usize,
    refazer: usize,
    /// Só quando a versão mudou desde o último estado: as células tocadas
    /// (`[linha, coluna, linha, coluna, …]`), ou `null` quando não se sabe
    /// onde. O gráfico mantém o cache se nenhuma cai no intervalo dele.
    #[serde(skip_serializing_if = "Option::is_none")]
    tocadas: Option<Json>,
}

/// O que todo comando que muda algo responde.
#[derive(Serialize)]
pub struct Estado {
    abas: Vec<EstadoAba>,
    nomes: Vec<Json>,
}

// ------------------------------------------------------------------ JSON

/// O estilo como veio do TypeScript (`Partial<Style>`, com `null` no lugar do
/// `undefined` que apaga).
pub fn estilo_de_json(v: &Json) -> Option<EstiloParcial> {
    let obj = v.as_object()?;
    let mut e = EstiloParcial::new();
    for (k, v) in obj {
        if let Some(c) = campo_de_json(v) {
            e.insert(k.clone(), c);
        }
    }
    Some(e)
}

fn campo_de_json(v: &Json) -> Option<Campo> {
    Some(match v {
        Json::Null => Campo::Nulo,
        Json::Bool(b) => Campo::Logico(*b),
        Json::Number(n) => Campo::Numero(n.as_f64()?),
        Json::String(s) => Campo::Texto(s.clone()),
        Json::Object(o) => Campo::Objeto(
            o.iter().filter_map(|(k, v)| Some((k.clone(), campo_de_json(v)?))).collect(),
        ),
        Json::Array(_) => return None,
    })
}

/// O caminho de volta: o estilo guardado, como o TypeScript o escreve.
pub fn estilo_para_json(e: &EstiloParcial) -> Json {
    Json::Object(e.iter().map(|(k, v)| (k.clone(), campo_para_json(v))).collect())
}

fn campo_para_json(c: &Campo) -> Json {
    match c {
        Campo::Nulo => Json::Null,
        Campo::Logico(b) => Json::Bool(*b),
        Campo::Numero(n) => {
            // Inteiro volta como inteiro: `fontSize: 12`, não `12.0`.
            if n.fract() == 0.0 && n.abs() < 9e15 {
                json!(*n as i64)
            } else {
                json!(*n)
            }
        }
        Campo::Texto(s) => Json::String(s.clone()),
        Campo::Objeto(o) => Json::Object(o.iter().map(|(k, v)| (k.clone(), campo_para_json(v))).collect()),
    }
}

fn rect_de_json(v: &Json) -> Result<Rect, String> {
    let n = |k: &str| {
        v.get(k)
            .and_then(Json::as_f64)
            .map(|x| x.max(0.0) as u32)
            .ok_or_else(|| format!("retângulo sem \"{k}\""))
    };
    let mut r = Rect::new(n("r0")?, n("c0")?, n("r1")?, n("c1")?);
    r.sheet = v.get("sheet").and_then(Json::as_str).map(str::to_string);
    Ok(r)
}

fn rect_para_json(r: &Rect) -> Json {
    let mut o = json!({ "r0": r.r0, "c0": r.c0, "r1": r.r1, "c1": r.c1 });
    if let Some(s) = &r.sheet {
        o["sheet"] = json!(s);
    }
    o
}

/// Uma caixa de referência da fórmula como o `RefBox` do TypeScript.
fn caixa_para_json(c: &feathersheet_core::formula::ref_drag::Caixa) -> Json {
    let mut j = json!({
        "start": c.start, "end": c.end, "rect": rect_para_json(&c.rect), "color": c.color,
        "locks": { "c0": c.locks.c0, "r0": c.locks.r0, "c1": c.locks.c1, "r1": c.locks.r1 },
    });
    if let Some(s) = &c.sheet {
        j["sheet"] = json!(s);
    }
    j
}

/// As travas (`{c0, r0, c1, r1}`) que a caixa mandou de volta.
fn travas_de_json(t: &Json) -> feathersheet_core::formula::ref_drag::Travas {
    let trava = |k: &str| t.get(k).and_then(Json::as_bool).unwrap_or(false);
    feathersheet_core::formula::ref_drag::Travas { c0: trava("c0"), r0: trava("r0"), c1: trava("c1"), r1: trava("r1") }
}

/// Uma célula `{row, col}` mandada pela grade.
fn celula_de_json(v: &Json, k: &str) -> Result<(u32, u32), String> {
    let n = |c: &str| v.get(c).and_then(Json::as_f64).map(|x| x.max(0.0) as u32);
    n("row").zip(n("col")).ok_or_else(|| format!("faltou a célula \"{k}\""))
}

/// As opções de busca como o `SearchOptions` do `ops.ts` as manda.
fn opcoes_de_busca(v: &Json) -> feathersheet_core::SearchOptions {
    let sim = |k: &str| v.get(k).and_then(Json::as_bool).unwrap_or(false);
    feathersheet_core::SearchOptions { match_case: sim("matchCase"), whole_cell: sim("wholeCell"), in_formulas: sim("inFormulas") }
}

/// A achada seguinte a `pos` na ordem de leitura (ou a anterior), dando a
/// volta no fim — com o índice dela na lista.
fn proxima_achada(achadas: &[feathersheet_core::SearchMatch], pos: (u32, u32), reverso: bool) -> Option<(usize, feathersheet_core::SearchMatch)> {
    if achadas.is_empty() {
        return None;
    }
    let k = if reverso {
        achadas.iter().rposition(|m| (m.row, m.col) < pos).unwrap_or(achadas.len() - 1)
    } else {
        achadas.iter().position(|m| (m.row, m.col) > pos).unwrap_or(0)
    };
    Some((k, achadas[k]))
}

fn snap_de_json(v: &Json) -> Option<Snap> {
    if v.is_null() {
        return None;
    }
    Some(Snap {
        raw: v.get("raw").and_then(Json::as_str).unwrap_or_default().to_string(),
        style: v.get("style").and_then(estilo_de_json).filter(|e| !e.is_empty()),
    })
}

/// Os argumentos de um comando, com as perguntas de sempre.
struct Args(Json);

impl Args {
    fn u32(&self, k: &str) -> Result<u32, String> {
        self.0
            .get(k)
            .and_then(Json::as_f64)
            .map(|x| x.max(0.0) as u32)
            .ok_or_else(|| format!("faltou o argumento \"{k}\""))
    }

    fn texto(&self, k: &str) -> Result<&str, String> {
        self.0.get(k).and_then(Json::as_str).ok_or_else(|| format!("faltou o argumento \"{k}\""))
    }

    fn logico(&self, k: &str) -> bool {
        self.0.get(k).and_then(Json::as_bool).unwrap_or(false)
    }

    fn get(&self, k: &str) -> &Json {
        self.0.get(k).unwrap_or(&Json::Null)
    }

    fn rect(&self, k: &str) -> Result<Rect, String> {
        rect_de_json(self.get(k))
    }
}

fn em_json<T: Serialize>(v: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(v).map_err(|e| e.to_string())
}

/// O `tipos` de `dividir_texto`: uma lista de `{"tipo": "geral"|"texto"|
/// "numero"|"data"|"porcentagem"|"pular", "formato"?: "dma"|"mda"|"amd"}`,
/// uma entrada por coluna resultante. Ausente ou mal formado vira `None` —
/// `aplicar_divisao` já trata isso como "geral" para toda coluna.
/// `OpcoesDivisao` a partir do JSON do assistente: `delimitadores` (lista de
/// tokens — geralmente um caractere, "Outros:" pode ser mais de um),
/// `consecutivos` (booleano) e `qualificador` (um único caractere, como
/// string — o primeiro `char` dela vale, o resto é ignorado).
fn opcoes_de_divisao(a: &Args) -> feathersheet_core::operacoes::ops::OpcoesDivisao {
    let delimitadores = a
        .get("delimitadores")
        .as_array()
        .map(|itens| itens.iter().filter_map(Json::as_str).map(str::to_string).collect())
        .unwrap_or_default();
    let consecutivos_como_um = a.get("consecutivos").as_bool().unwrap_or(false);
    let qualificador_texto = a.get("qualificador").as_str().and_then(|s| s.chars().next());
    feathersheet_core::operacoes::ops::OpcoesDivisao { delimitadores, consecutivos_como_um, qualificador_texto }
}

fn tipos_de_divisao(json: &Json) -> Option<Vec<TipoColunaDivisao>> {
    let itens = json.as_array()?;
    Some(
        itens
            .iter()
            .map(|item| {
                let formato = || match item.get("formato").and_then(Json::as_str) {
                    Some("mda") => FormatoDeData::MesDiaAno,
                    Some("amd") => FormatoDeData::AnoMesDia,
                    _ => FormatoDeData::DiaMesAno,
                };
                match item.get("tipo").and_then(Json::as_str) {
                    Some("texto") => TipoColunaDivisao::Texto,
                    Some("numero") => TipoColunaDivisao::Numero,
                    Some("data") => TipoColunaDivisao::Data(formato()),
                    Some("porcentagem") => TipoColunaDivisao::Porcentagem,
                    Some("pular") => TipoColunaDivisao::Pular,
                    _ => TipoColunaDivisao::Geral,
                }
            })
            .collect(),
    )
}

// ----------------------------------------------------------------- a ponte

impl Ponte {
    pub fn com_pasta(pasta: Workbook) -> Self {
        Ponte {
            pasta,
            ocultas: HashMap::new(),
            condicionais: HashMap::new(),
            buscas: HashMap::new(),
            versoes_informadas: HashMap::new(),
            comando_preciso: false,
            vinculos: HashMap::new(),
            nomes_dos_vinculos: HashMap::new(),
            consultas: HashMap::new(),
            proxima_consulta: 1,
        }
    }

    /// Troca a pasta inteira — e esquece o que era da anterior.
    fn trocar_pasta(&mut self, pasta: Workbook) {
        self.pasta = pasta;
        self.ocultas.clear();
        self.condicionais.clear();
        self.buscas.clear();
        self.versoes_informadas.clear();
        self.vinculos.clear();
        self.nomes_dos_vinculos.clear();
    }

    /// Abre um `.fshz` e troca a pasta inteira por ele.
    pub fn abrir_fshz(&mut self, bytes: &[u8]) -> Result<Vec<u8>, String> {
        let aberto = PastaAberta::ler(bytes)?;
        self.instalar(aberto)
    }

    /// Troca a pasta pela que foi lida — a parte que precisa da ponte. Ler é
    /// separado para o aplicativo poder desmontar o arquivo **sem** segurar a
    /// ponte: enquanto ela está presa, cada bloco que a grade pede espera.
    ///
    /// Responde o estado, os metadados de cada aba (no formato do FSH2, que é
    /// o que `io/metadados-aba.ts` entende), os nomes e a aba ativa. As
    /// células ficam aqui: o TypeScript só vai buscar o bloco que desenhar.
    pub fn instalar(&mut self, aberto: PastaAberta) -> Result<Vec<u8>, String> {
        self.trocar_pasta(aberto.pasta);
        let estado = self.estado();
        let metadados: Vec<Json> = (0..self.pasta.len())
            .filter_map(|i| self.pasta.aba(i).map(|v| metadados_para_json(&v.sheet)))
            .collect();
        em_json(&json!({ "estado": estado, "metadados": metadados, "nomes": aberto.nomes, "ativa": aberto.ativa }))
    }

    /// Uma aba nova, no fim, já com as linhas coladas — é como um CSV entra.
    /// Sem desfazer: abrir um arquivo não é uma edição que se volte.
    ///
    /// Responde o `id` da aba, o estado e quantas linhas o arquivo tinha (o
    /// que passar do teto de linhas da aba fica de fora, e a interface avisa).
    /// Uma aba nova a partir do texto de um CSV, cortado e colado em lotes
    /// ([`feathersheet_io::csv::dividir_csv_em_lotes`]): o arquivo inteiro nunca
    /// vira uma lista de `String` de uma vez (8 GB num CSV de 800 MB).
    /// Abre um CSV inteiro como aba nova, segurando a ponte do começo ao fim.
    /// O aplicativo usa os três passos abaixo, que soltam a ponte entre os lotes.
    pub fn inserir_aba_de_csv(&mut self, nome: String, texto: &str) -> Result<Vec<u8>, String> {
        self.inserir_aba_de_csv_com_separador(nome, texto, None)
    }

    pub fn inserir_aba_de_csv_com_separador(&mut self, nome: String, texto: &str, separador: Option<char>) -> Result<Vec<u8>, String> {
        let delim = separador.unwrap_or_else(|| feathersheet_io::csv::detectar_separador(texto));
        let id = self.csv_comecar(nome);
        let mut linha = 0u32;
        feathersheet_io::csv::dividir_csv_em_lotes(texto, delim, feathersheet_io::csv::LINHAS_POR_LOTE, |lote| {
            self.csv_colar_lote(id, linha, lote);
            linha += lote.len() as u32;
        });
        self.csv_concluir(id, linha)
    }

    /// Cria a aba que vai receber o CSV e devolve o `id` dela.
    pub fn csv_comecar(&mut self, nome: String) -> u32 {
        let i = self.pasta.criar_aba(nome);
        self.pasta.aba(i).map(|v| v.id).unwrap_or(0)
    }

    /// Cola um lote de linhas do CSV a partir de `inicio`. Pela `id`, e não
    /// pelo índice: entre um lote e outro a pessoa pode mexer nas abas.
    pub fn csv_colar_lote(&mut self, id: u32, inicio: u32, lote: &[Vec<String>]) {
        if let Some(i) = self.pasta.indice_do_id(id) {
            self.pasta.editar(i, |s| s.paste(inicio, 0, lote, false));
        }
    }

    /// Fecha a importação: responde o `id`, o estado e quantas linhas vieram.
    pub fn csv_concluir(&mut self, id: u32, linhas: u32) -> Result<Vec<u8>, String> {
        let estado = self.estado();
        em_json(&json!({ "id": id, "estado": estado, "linhas": linhas }))
    }

    pub fn inserir_aba_de_linhas(&mut self, nome: String, linhas: &[Vec<String>]) -> Result<Vec<u8>, String> {
        let i = self.pasta.criar_aba(nome);
        let id = self.pasta.aba(i).map(|v| v.id).unwrap_or(0);
        self.pasta.editar(i, |s| s.paste(0, 0, linhas, false));
        let estado = self.estado();
        em_json(&json!({ "id": id, "estado": estado, "linhas": linhas.len() }))
    }

    /// O `.fshz` da pasta inteira, montado daqui mesmo — nenhuma célula volta
    /// ao TypeScript para ser gravada. Os metadados de cada aba precisam ter
    /// chegado antes (`metadados_definir`).
    pub fn gravar_fshz(&self, ativa: u32) -> Result<Vec<u8>, String> {
        feathersheet_io::fshz_write::gerar_fshz2(&self.retrato(ativa))
    }

    /// Os XML do `.xlsx` da pasta inteira, já escritos. É a parte da
    /// exportação que precisa da pasta; comprimir
    /// ([`feathersheet_io::zip_write::zipar`]) fica para depois de soltar a ponte.
    pub fn partes_xlsx(&self) -> Vec<(String, Vec<u8>)> {
        feathersheet_io::xlsx_write::montar_partes(&self.pasta)
    }

    /// Uma aba em CSV (`separador` `,`) ou TSV (`\t`), já formatada como a
    /// grade mostra. `aba` é o `id` que o TypeScript conhece.
    pub fn exportar_delimitado(&mut self, aba: u32, separador: char) -> Result<Vec<u8>, String> {
        let i = self.pasta.indice_do_id(aba).ok_or_else(|| format!("não existe a aba {aba}"))?;
        feathersheet_io::xlsx_write::gerar_delimitado(&mut self.pasta, i, separador)
    }

    /// As colunas copiadas para gravar. É a única parte da gravação que
    /// precisa da ponte; comprimir (o caro) fica para depois de soltá-la.
    pub fn retrato(&self, ativa: u32) -> feathersheet_io::fshz_write::PastaBruta {
        feathersheet_io::da_workbook::da_workbook(&self.pasta, ativa)
    }

    /// O índice da aba pelo `id` que o TypeScript conhece.
    fn indice(&self, a: &Args) -> Result<usize, String> {
        let id = a.u32("aba")?;
        self.pasta.indice_do_id(id).ok_or_else(|| format!("não existe a aba {id}"))
    }

    fn sheet_mut(&mut self, a: &Args) -> Result<&mut Sheet, String> {
        let i = self.indice(a)?;
        Ok(&mut self.pasta.aba_mut(i).expect("índice acabou de ser achado").sheet)
    }

    /// Recusa a escrita de uma célula (`Err`) quando uma validação "recusar"
    /// cobre `(row, col)` e `texto` não passa nela. Freio de segurança do
    /// funil único de escrita — a mensagem informativa (avisar ou recusar,
    /// com o motivo) é o comando `validacao_verificar`, que a interface
    /// chama antes; isto aqui não deixa passar mesmo que ela não chame.
    fn recusar_se_invalido(&self, a: &Args, row: u32, col: u32, texto: &str) -> Result<(), String> {
        let i = self.indice(a)?;
        let va = self.pasta.aba(i).expect("achada").sheet.va.clone().unwrap_or_default();
        if va.is_empty() {
            return Ok(());
        }
        let validacoes = validacoes_de_json(&va);
        let Some(v) = validacoes.iter().find(|v| v.ranges.iter().any(|rect| rect.contains(row, col))) else {
            return Ok(());
        };
        if v.quando != feathersheet_core::operacoes::validation::QuandoValidacao::Recusar {
            return Ok(());
        }
        let valor = if texto.is_empty() { feathersheet_core::Value::Empty } else { feathersheet_core::Value::Text(texto.to_string()) };
        let pasta = &self.pasta;
        let resultado = feathersheet_core::operacoes::validation::validar(&valor, &v.regra, Some(|r: u32, c: u32| pasta.valor(i, r, c)));
        if resultado.ok {
            return Ok(());
        }
        Err(v.mensagem.clone().unwrap_or_else(|| resultado.motivo.unwrap_or_default()))
    }

    /// O estado da pasta inteira. São poucas abas: vai tudo, sempre.
    pub fn estado(&mut self) -> Estado {
        let mut abas = Vec::with_capacity(self.pasta.len());
        for i in 0..self.pasta.len() {
            let viva = self.pasta.aba_mut(i).expect("dentro do tamanho");
            let (versao, id) = (viva.versao, viva.id);
            let s = &mut viva.sheet;
            let rastro = s.rastro.borrow_mut().tirar();
            let tocadas = if self.versoes_informadas.insert(id, versao) == Some(versao) {
                None
            } else {
                Some(match rastro {
                    Some(chaves) if self.comando_preciso => {
                        let mut v = Vec::with_capacity(chaves.len() * 2);
                        for k in chaves {
                            v.push(feathersheet_core::celulas::refs::key_row(k));
                            v.push(feathersheet_core::celulas::refs::key_col(k));
                        }
                        json!(v)
                    }
                    _ => Json::Null,
                })
            };
            abas.push(EstadoAba {
                tocadas,
                id,
                nome: s.name.clone(),
                versao,
                max_row: s.max_row_ts(),
                max_col: s.max_col_ts(),
                pode_desfazer: s.can_undo(),
                pode_refazer: s.can_redo(),
                desfazer: s.profundidade_do_historico(),
                refazer: s.profundidade_do_refazer(),
            });
        }
        let nomes = self
            .pasta
            .nomeados()
            .iter()
            .map(|n| json!({ "name": n.nome, "rect": rect_para_json(&n.area) }))
            .collect();
        Estado { abas, nomes }
    }

    fn responder_estado(&mut self) -> Result<Vec<u8>, String> {
        let e = self.estado();
        em_json(&e)
    }

    /// Muda a aba do comando, sobe a versão e responde o estado.
    fn editar<F>(&mut self, a: &Args, acao: F) -> Result<Vec<u8>, String>
    where
        F: FnOnce(&mut Sheet),
    {
        let i = self.indice(a)?;
        self.pasta.editar(i, acao);
        self.responder_estado()
    }

    /// As perguntas novas de todas as abas (`{ aba, chave }`), que passam a
    /// estar "em curso": não voltam na próxima vez.
    fn tomar_pendentes(&self) -> Vec<Json> {
        let mut v = Vec::new();
        for i in 0..self.pasta.len() {
            let viva = self.pasta.aba(i).expect("dentro do tamanho");
            for chave in viva.sheet.assincronos.borrow_mut().tomar_pendentes() {
                v.push(json!({ "aba": viva.id, "chave": chave }));
            }
        }
        v
    }

    /// A resposta chegou (ou foi jogada fora): as células que perguntaram
    /// ficam sujas, quem depende delas também, e elas são recalculadas na
    /// frente — é o que faz a matriz derramar e o "atualizar" perguntar de
    /// novo, como o `onAsyncSettled`/`refreshAsync` do TypeScript. Ninguém
    /// perguntou: nada muda, nem a versão.
    fn recalcular_quem_perguntou(&mut self, i: usize, quem: &[u64]) {
        if quem.is_empty() {
            return;
        }
        self.pasta.editar(i, |s| s.sujar_celulas(quem));
        for &k in quem {
            let (r, c) = Sheet::key_to_coord(k);
            self.pasta.valor(i, r, c);
        }
    }

    /// Carrega as pastas externas pedidas e relê as que mudaram no disco.
    /// Só `.fshz` por ora. Arquivo que não existe fica de fora (a fórmula
    /// continua em `#REF!`) e é tentado de novo na próxima vez.
    fn atualizar_vinculos(&mut self, base: &str, ao_vivo: &HashSet<String>) -> bool {
        let carregados = self.pasta.externos_carregados();
        let mut nomes = self.pasta.externos_pedidos();
        nomes.extend(carregados.iter().cloned());
        nomes.sort();
        nomes.dedup();
        let mut mudou = false;
        for nome in nomes {
            let caminho = {
                let p = std::path::Path::new(&nome);
                if p.is_absolute() || base.is_empty() { p.to_path_buf() } else { std::path::Path::new(base).join(p) }
            };
            let chave = caminho.to_string_lossy().into_owned();
            // Antes do disco: a janela que tem o arquivo aberto responde pelo
            // caminho, mesmo que ele nunca tenha sido gravado.
            self.nomes_dos_vinculos.insert(chave.clone(), nome.clone());
            // Aberto em outra janela: quem manda é ela (`vinculo_retrato` e
            // `vinculo_valores`), e o disco pode estar atrás do que já chegou.
            if carregados.contains(&nome) && ao_vivo.contains(&chave) {
                continue;
            }
            let Ok(meta) = std::fs::metadata(&caminho) else { continue };
            let data = meta.modified().ok();
            if carregados.contains(&nome) && self.vinculos.get(&chave) == Some(&data) {
                continue;
            }
            let Ok(bytes) = std::fs::read(&caminho) else { continue };
            let Ok(aberta) = PastaAberta::ler(&bytes) else { continue };
            let abas = feathersheet_core::planilha::vinculo::retrato_de_valores(&aberta.pasta);
            mudou |= self.pasta.instalar_externo(&nome, abas);
            self.vinculos.insert(chave, data);
        }
        mudou
    }

    /// Atende um comando. `corpo` é JSON (vazio vale `{}`).
    ///
    /// Em volta dele, o rastro das células tocadas: um comando fora de
    /// [`COMANDOS_PRECISOS`] que mudou uma aba sem contar ao TypeScript
    /// deixa o rastro dela desconhecido, para o próximo estado não entregar
    /// uma lista incompleta.
    pub fn chamar(&mut self, comando: &str, corpo: &[u8]) -> Result<Vec<u8>, String> {
        self.comando_preciso = COMANDOS_PRECISOS.contains(&comando);
        let resposta = self.atender(comando, corpo);
        if !self.comando_preciso {
            for i in 0..self.pasta.len() {
                let viva = self.pasta.aba(i).expect("dentro do tamanho");
                if self.versoes_informadas.get(&viva.id) != Some(&viva.versao) {
                    viva.sheet.rastro.borrow_mut().desconhecer();
                }
            }
        }
        self.comando_preciso = false;
        resposta
    }

    fn atender(&mut self, comando: &str, corpo: &[u8]) -> Result<Vec<u8>, String> {
        // O arquivo vem cru, no corpo: nada de JSON de bytes (um número
        // decimal por byte quadruplicava o arquivo na travessia).
        if comando == "pasta_abrir_fshz" {
            return self.abrir_fshz(corpo);
        }
        if comando == "pasta_abrir_xlsx" {
            // O leitor mora no `sheet-io` (`xlsx_read`) e produz o formato do
            // `loadJSON`, que vira pasta aqui mesmo.
            let livro = feathersheet_io::xlsx_read::ler_xlsx_bytes(corpo.to_vec())?;
            return self.instalar(PastaAberta::de_json(&livro));
        }
        let json: Json = if corpo.is_empty() {
            Json::Object(Default::default())
        } else {
            serde_json::from_slice(corpo).map_err(|e| format!("corpo inválido em \"{comando}\": {e}"))?
        };
        let a = Args(json);
        match comando {
            // ------------------------------------------------------ pasta
            // ----------------------------------- escrever fórmula (sem aba)
            "formula_referencias" => {
                // As referências coloridas do texto e as caixas que a grade
                // desenha (`formula/assistente.rs`).
                use feathersheet_core::formula::assistente::{color_references, reference_boxes};
                let texto = a.texto("texto")?;
                em_json(&json!({
                    "referencias": color_references(texto).iter().map(|r| json!({
                        "text": r.text, "start": r.start, "end": r.end, "color": r.color,
                    })).collect::<Vec<_>>(),
                    "caixas": reference_boxes(texto).iter().map(caixa_para_json).collect::<Vec<_>>(),
                }))
            }
            "formula_caixas_visiveis" => {
                // As caixas que valem na aba à vista (`boxesForSheet`): as que
                // nomeiam a aba, e as sem nome quando a fórmula é escrita nela.
                use feathersheet_core::formula::ref_drag::{boxes_for_sheet, reference_boxes};
                let caixas = reference_boxes(a.texto("texto")?);
                let visiveis = boxes_for_sheet(&caixas, a.texto("aba")?, a.logico("naAbaDaFormula"));
                em_json(&visiveis.into_iter().map(caixa_para_json).collect::<Vec<_>>())
            }
            "formula_arrastar" => {
                // Um passo do arrasto da caixa tracejada: a área nova (mover
                // pelo deslocamento ou esticar até o alvo) já escrita na
                // fórmula, com as travas (`formula/ref_drag.rs`).
                use feathersheet_core::formula::ref_drag::{drag_rect, Pegada};
                use feathersheet_core::formula::assistente::rewrite_box;
                let pegada = match a.texto("pegada")? {
                    "move" => Pegada::Mover,
                    "resize" => Pegada::Redimensionar,
                    outra => return Err(format!("pegada desconhecida: \"{outra}\"")),
                };
                let rect = drag_rect(&a.rect("rect")?, pegada, celula_de_json(a.get("de"), "de")?, celula_de_json(a.get("alvo"), "alvo")?);
                let texto = rewrite_box(a.texto("texto")?, a.u32("inicio")? as usize, a.u32("fim")? as usize, travas_de_json(a.get("travas")), &rect);
                em_json(&texto)
            }
            "formula_cursor" => {
                // O que o cursor pede: sugestões, a função em volta (com a
                // ajuda) e se um clique ali vira referência.
                use feathersheet_core::formula::assistente::{accepts_reference, active_call, ajuda, suggest_at, token_before};
                let texto = a.texto("texto")?;
                let cursor = a.u32("cursor")? as usize;
                let ajuda_json = |h: Option<&feathersheet_core::funcoes::function_help::FunctionHelp>| match h {
                    Some(h) => json!({ "args": h.args, "doc": h.doc }),
                    None => Json::Null,
                };
                let chamada = active_call(texto, cursor);
                em_json(&json!({
                    "token": token_before(texto, cursor).map(|t| json!({ "text": t.text, "start": t.start, "end": t.end })),
                    "sugestoes": suggest_at(texto, cursor, 8, feathersheet_core::get_locale_config().code.starts_with("en")).iter().map(|s| json!({
                        "name": s.name, "help": ajuda_json(s.help), "signature": s.signature,
                    })).collect::<Vec<_>>(),
                    "ajudaDaChamada": ajuda_json(chamada.as_ref().and_then(|c| ajuda(&c.name))),
                    "chamada": chamada.map(|c| json!({ "name": c.name, "argIndex": c.arg_index, "open": c.open })),
                    "aceitaReferencia": accepts_reference(texto, cursor),
                }))
            }
            "formula_reescrever" => {
                // A referência arrastada escrita de volta, com as travas.
                use feathersheet_core::formula::assistente::rewrite_box;
                let travas = travas_de_json(a.get("travas"));
                em_json(&rewrite_box(a.texto("texto")?, a.u32("inicio")? as usize, a.u32("fim")? as usize, travas, &a.rect("rect")?))
            }
            "funcoes_catalogo" => {
                // O catálogo das funções, pedido uma vez: categorias, ajuda,
                // apelidos e as que não têm ajuda.
                use feathersheet_core::formula::assistente::{aliases_of, categorias, functions_without_help};
                use feathersheet_core::funcoes::function_help::all;
                let nomes = feathersheet_core::funcoes::functions::function_names();
                let mut ajuda = serde_json::Map::new();
                for (nome, h) in all() {
                    ajuda.insert(nome.to_string(), json!({ "args": h.args, "doc": h.doc }));
                }
                let mut apelidos = serde_json::Map::new();
                let mut canonicos = serde_json::Map::new();
                for (nome, _) in all() {
                    apelidos.insert(nome.to_string(), json!(aliases_of(nome)));
                }
                for &(apelido, real) in feathersheet_core::funcoes::functions::aliases() {
                    if nomes.contains(&real) {
                        canonicos.insert(apelido.to_string(), json!(real));
                    }
                }
                em_json(&json!({
                    "categorias": categorias().iter().map(|(c, n)| json!([c, n])).collect::<Vec<_>>(),
                    "ajuda": ajuda,
                    "apelidos": apelidos,
                    "canonicos": canonicos,
                    "semAjuda": functions_without_help(),
                    "nomes": nomes,
                }))
            }
            // ------------------------------------------- OpenQuery (sem aba)
            "csv_dividir" => {
                // O texto de um CSV/TSV em linhas e campos, para o OpenQuery.
                // Sem `separador`, ele é descoberto (`detectar_separador`).
                let texto = a.texto("texto")?;
                let sep = match a.get("separador").as_str().and_then(|s| s.chars().next()) {
                    Some(c) => c,
                    None => feathersheet_io::csv::detectar_separador(texto),
                };
                em_json(&feathersheet_io::csv::dividir_csv(texto, sep))
            }
            "consulta_fonte" => {
                // Guarda uma tabela de origem (`{ cabecalho, linhas }`) e
                // responde o número dela.
                let id = self.proxima_consulta;
                self.proxima_consulta += 1;
                self.consultas.insert(id, tabela_de_json(&a.0));
                em_json(&json!({ "id": id }))
            }
            "consulta_esquecer" => {
                for id in a.get("ids").as_array().into_iter().flatten().filter_map(Json::as_u64) {
                    self.consultas.remove(&(id as u32));
                }
                em_json(&json!({ "ok": true }))
            }
            "consulta_executar" => {
                // Os passos sobre a origem `fonte`; `fontes` dá o número de
                // cada tabela que um passo junta ou acrescenta pelo nome.
                let fonte = self.consultas.get(&a.u32("fonte")?).cloned().ok_or("origem da consulta desconhecida")?;
                let mut fontes = HashMap::new();
                for (nome, id) in a.get("fontes").as_object().into_iter().flatten() {
                    if let Some(t) = id.as_u64().and_then(|id| self.consultas.get(&(id as u32))) {
                        fontes.insert(nome.clone(), t.clone());
                    }
                }
                let passos: Vec<feathersheet_core::Passo> = a
                    .get("passos")
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(passo_de_json)
                    .collect::<Result<_, _>>()?;
                let r = feathersheet_core::executar_query(fonte, &passos, &fontes);
                em_json(&json!({
                    "cabecalho": r.cabecalho,
                    "linhas": r.linhas.iter().map(|l| l.iter().map(valor_de_celula_para_json).collect::<Vec<_>>()).collect::<Vec<_>>(),
                }))
            }
            "pasta_nova" => {
                self.trocar_pasta(Workbook::new());
                self.responder_estado()
            }
            "pasta_estado" => self.responder_estado(),
            "pasta_vazia" => {
                // Sem aba nenhuma: quem carrega um arquivo cria as dele.
                self.trocar_pasta(Workbook::vazia());
                self.responder_estado()
            }
            "aba_zerar" => {
                // O `loadJSON` de uma aba: tudo embora, menos o nome.
                self.editar(&a, |s| {
                    let nome = std::mem::take(&mut s.name);
                    *s = Sheet::new(nome);
                })
            }
            "aba_inserir" => {
                let nome = a.texto("nome")?.to_string();
                let em = a.u32("em").unwrap_or(self.pasta.len() as u32) as usize;
                let i = self.pasta.inserir_aba(nome, em);
                let id = self.pasta.aba(i).map(|v| v.id).unwrap_or(0);
                let estado = self.estado();
                em_json(&json!({ "id": id, "estado": estado }))
            }
            "aba_remover" => {
                let i = self.indice(&a)?;
                let ok = self.pasta.remover_aba(i);
                let estado = self.estado();
                em_json(&json!({ "ok": ok, "estado": estado }))
            }
            "aba_renomear" => {
                let i = self.indice(&a)?;
                let ok = self.pasta.renomear_aba(i, a.texto("nome")?);
                let estado = self.estado();
                em_json(&json!({ "ok": ok, "estado": estado }))
            }
            "aba_mover" => {
                self.pasta.mover_aba(a.u32("de")? as usize, a.u32("para")? as usize);
                self.responder_estado()
            }
            "nome_definir" => {
                self.pasta.definir_nome(a.texto("nome")?, a.rect("rect")?);
                self.responder_estado()
            }
            "nome_remover" => {
                let ok = self.pasta.remover_nome(a.texto("nome")?);
                let estado = self.estado();
                em_json(&json!({ "ok": ok, "estado": estado }))
            }
            "aba_de_csv" => {
                // O texto do arquivo inteiro, cortado aqui. No aplicativo quem
                // chama é o `pasta_abrir_csv_disco`, que lê do disco; este
                // comando é a mesma porta para quem já tem o texto (testes).
                let texto = a.texto("texto")?.to_string();
                self.inserir_aba_de_csv_com_separador(a.texto("nome")?.to_string(), &texto, a.get("separador").as_str().and_then(|s| s.chars().next()))
            }
            "pasta_gravar_fshz" => self.gravar_fshz(a.u32("ativa").unwrap_or(0)),
            // Os bytes do arquivo, crus. No aplicativo quem exporta é o
            // `pasta_exportar_*_disco`, que nem os traz para cá; estes servem
            // o navegador (`npm run dev`) e os testes.
            "pasta_exportar_xlsx" => feathersheet_io::zip_write::zipar(self.partes_xlsx()),
            "aba_exportar_delimitado" => {
                let sep = match a.texto("separador").unwrap_or(",") {
                    "," => ',', ";" => ';', "\t" => '\t',
                    _ => return Err("Separador deve ser vírgula, ponto e vírgula ou TAB".into()),
                };
                self.exportar_delimitado(a.u32("aba")?, sep)
            }
            "metadados_definir" => {
                let s = self.sheet_mut(&a)?;
                definir_metadados(s, &a.0);
                self.responder_estado()
            }
            "metadados_obter" => {
                let i = self.indice(&a)?;
                em_json(&metadados_para_json(&self.pasta.aba(i).expect("achada").sheet))
            }

            // ------------------------------------- valores que respondem depois
            // O `async.ts` dividido em dois: o motor anota a pergunta e mostra
            // `#CALCULANDO!` (`planilha/assincrono.rs`); a interface busca na
            // rede ou roda a função da macro e devolve a resposta.
            "tarefas_pendentes" => {
                // As perguntas novas de todas as abas. A interface só pede
                // quando um bloco trouxe `#CALCULANDO!` (regra 16).
                em_json(&json!({ "pendentes": self.tomar_pendentes() }))
            }
            "tarefa_resolver" => {
                // `{ aba, chave, texto }` (o texto cru da rede, interpretado
                // pela chave em `sheet-io/web.rs`), `{ …, valor }` (um valor
                // ou uma tabela prontos, da função de macro) ou `{ …, erro }`
                // (o motivo da falha: vira `#ERROR!`, como a promessa
                // rejeitada do TypeScript).
                let i = self.indice(&a)?;
                let chave = a.texto("chave")?.to_string();
                let resposta = if let Some(texto) = a.get("texto").as_str() {
                    feathersheet_io::web::interpretar(&chave, texto)
                } else if let Some(erro) = a.get("erro").as_str() {
                    feathersheet_core::planilha::assincrono::Resposta::falha(erro)
                } else {
                    resposta_de_valor(a.get("valor"))
                };
                let quem = self.pasta.aba(i).expect("achada").sheet.assincronos.borrow_mut().resolver(&chave, resposta);
                self.recalcular_quem_perguntou(i, &quem);
                let pendentes = self.tomar_pendentes();
                let estado = self.estado();
                em_json(&json!({ "estado": estado, "pendentes": pendentes }))
            }
            "tarefas_atualizar" => {
                // O "atualizar" (`refreshAsync`): esquece as respostas da
                // `aba` (ou de todas, sem ela) e pergunta de novo.
                let abas = match a.get("aba") {
                    Json::Null => 0..self.pasta.len(),
                    _ => {
                        let i = self.indice(&a)?;
                        i..i + 1
                    }
                };
                for i in abas {
                    let quem = self.pasta.aba(i).expect("dentro do tamanho").sheet.assincronos.borrow_mut().atualizar();
                    self.recalcular_quem_perguntou(i, &quem);
                }
                let pendentes = self.tomar_pendentes();
                let estado = self.estado();
                em_json(&json!({ "estado": estado, "pendentes": pendentes }))
            }
            "funcoes_externas" => {
                // As funções criadas por macro que o motor conhece.
                let mut nomes: Vec<&String> = self.pasta.funcoes_externas.iter().collect();
                nomes.sort();
                em_json(&json!({ "nomes": nomes }))
            }
            "funcoes_externas_definir" => {
                // A lista inteira (`nomes`) e as que ganharam corpo novo
                // (`esquecer`): a resposta guardada delas era do corpo velho.
                let nomes: Vec<String> =
                    a.get("nomes").as_array().into_iter().flatten().filter_map(Json::as_str).map(str::to_string).collect();
                self.pasta.definir_funcoes_externas(nomes);
                for nome in a.get("esquecer").as_array().into_iter().flatten().filter_map(Json::as_str) {
                    let prefixo = format!("macro:{}\u{0}", nome.to_uppercase());
                    for i in 0..self.pasta.len() {
                        let quem = self.pasta.aba(i).expect("dentro do tamanho").sheet.assincronos.borrow_mut().esquecer(&prefixo);
                        self.recalcular_quem_perguntou(i, &quem);
                    }
                }
                self.responder_estado()
            }

            // ------------------------------------------------------ leitura
            "bloco" => {
                let i = self.indice(&a)?;
                Ok(feathersheet_core::planilha::bloco::montar(
                    &self.pasta,
                    i,
                    a.u32("r0")?,
                    a.u32("c0")?,
                    a.u32("r1")?,
                    a.u32("c1")?,
                    a.logico("texto"),
                ))
            }
            "ocultas_definir" => {
                // `faixas`: [[início, fim], …] das linhas de altura 0.
                let id = a.u32("aba")?;
                self.indice(&a)?;
                let faixas: Vec<(u32, u32)> = a
                    .get("faixas")
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|f| Some((f.get(0)?.as_f64()?.max(0.0) as u32, f.get(1)?.as_f64()?.max(0.0) as u32)))
                    .collect();
                self.ocultas.insert(id, FaixasOcultas::new(faixas));
                em_json(&json!({ "ok": true }))
            }
            "borda" => {
                // Ctrl+seta: o salto inteiro numa ida (`Sheet::borda_dos_dados`).
                let int = |k: &str| a.get(k).as_f64().map(|x| x as i32).ok_or_else(|| format!("faltou o argumento \"{k}\""));
                let (dr, dc) = (int("dr")?, int("dc")?);
                let (row, col) = (a.u32("linha")?, a.u32("coluna")?);
                let (lr, lc) = (a.u32("limiteLinha")?, a.u32("limiteColuna")?);
                let destino = self.sheet_mut(&a)?.borda_dos_dados(row, col, dr, dc, lr, lc);
                em_json(&json!([destino.0, destino.1]))
            }
            "valores_distintos" => {
                // A lista de caixinhas do filtro: os valores distintos da
                // coluna, na ordem em que aparecem, até `limite`. Numa coluna
                // de IDs com dois milhões de valores, a lista vem cortada e
                // `cortado` avisa.
                let i = self.indice(&a)?;
                let col = a.u32("coluna")?;
                let (r0, r1) = (a.u32("r0")?, a.u32("r1")?);
                let limite = a.u32("limite").unwrap_or(10_000) as usize;
                let pasta = &self.pasta;
                let mut vistos = std::collections::HashSet::new();
                let mut valores = Vec::new();
                let mut cortado = false;
                let fim = r1.min(pasta.aba(i).map(|v| v.sheet.max_row).unwrap_or(0));
                for r in r0..=fim.max(r0) {
                    let v = pasta.valor(i, r, col);
                    let chave = (std::mem::discriminant(&v), feathersheet_core::operacoes::filtro::texto_js(&v));
                    if vistos.contains(&chave) {
                        continue;
                    }
                    if valores.len() >= limite {
                        cortado = true;
                        break;
                    }
                    vistos.insert(chave);
                    valores.push(valor_para_json(&v));
                }
                em_json(&json!({ "valores": valores, "cortado": cortado }))
            }
            "filtro_avaliar" => {
                // As linhas que o filtro esconde, em faixas (`filtro.rs`).
                let i = self.indice(&a)?;
                let (r0, r1) = (a.u32("r0")?, a.u32("r1")?);
                let filtros: Vec<(u32, feathersheet_core::operacoes::filtro::Criterio)> = a
                    .get("filtros")
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|f| Some((f.get("col")?.as_u64()? as u32, criterio_de_json(f)?)))
                    .collect();
                let pasta = &self.pasta;
                let fim = r1.min(pasta.aba(i).map(|v| v.sheet.max_row).unwrap_or(0));
                let faixas = if fim < r0 {
                    Vec::new()
                } else {
                    feathersheet_core::operacoes::filtro::linhas_escondidas(r0, fim, &filtros, &|r, c| pasta.valor(i, r, c))
                };
                em_json(&faixas.iter().map(|(x, y)| [*x, *y]).collect::<Vec<_>>())
            }
            "grafico_dados" => {
                // Os números do gráfico, calculados lendo a aba viva
                // (`chart.rs`). Antes o TypeScript lia a planilha inteira pela
                // ponte a cada vez que a aba aparecia: 7,8 s na planilha de
                // dois milhões de linhas (bug de 23/09/2026).
                let i = self.indice(&a)?;
                let cfg: feathersheet_core::grafico::chart::ChartConfig = serde_json::from_value(a.get("config").clone())
                    .map_err(|e| format!("configuração de gráfico inválida: {e}"))?;
                let pasta = &self.pasta;
                let d = feathersheet_core::grafico::chart::extract_chart_data_with(&|r, c| pasta.valor(i, r, c), &cfg);
                // NaN não cabe em JSON: vai `null`, e o TypeScript o devolve a NaN.
                let num = |n: f64| if n.is_finite() { json!(n) } else { Json::Null };
                let mut out = json!({
                    "title": d.title,
                    "categories": d.categories,
                    "series": d.series.iter().map(|s| json!({
                        "name": s.name,
                        "values": s.values.iter().map(|&v| num(v)).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                });
                if let Some(x) = &d.x_values {
                    out["xValues"] = Json::Array(x.iter().map(|&v| num(v)).collect());
                }
                if let Some(z) = &d.z_values {
                    out["zValues"] = Json::Array(z.iter().map(|&v| num(v)).collect());
                }
                if let Some(am) = d.amostrado {
                    out["amostrado"] = json!({ "disponiveis": am.disponiveis, "usados": am.usados });
                }
                // A linha de tendência e a média móvel, prontas para desenhar,
                // quando o gráfico as pede (antes eram contadas no TypeScript).
                let estilo = &a.get("config")["style"];
                if estilo.get("trendline").and_then(Json::as_bool).unwrap_or(false) {
                    use feathersheet_core::grafico::chart::{compute_linear_trendline, compute_moving_average};
                    let reta = |t: Option<feathersheet_core::grafico::chart::Tendencia>| match t {
                        Some(t) => json!({ "slope": num(t.slope), "intercept": num(t.intercept), "rSquared": num(t.r_squared) }),
                        None => Json::Null,
                    };
                    // Por posição (barras, linhas) e pelo X (dispersão: o
                    // `xValues`, ou a primeira série quando não há).
                    let xs: Vec<f64> = d.x_values.clone().or_else(|| d.series.first().map(|s| s.values.clone())).unwrap_or_default();
                    out["tendencias"] = Json::Array(d.series.iter().map(|s| {
                        let indices: Vec<f64> = (0..s.values.len()).map(|i| i as f64).collect();
                        reta(compute_linear_trendline(&indices, &s.values))
                    }).collect());
                    out["tendenciasX"] = Json::Array(d.series.iter().map(|s| reta(compute_linear_trendline(&xs, &s.values))).collect());
                    if let Some(primeira) = d.series.first() {
                        let periodo = estilo.get("trendlinePeriod").and_then(Json::as_f64).unwrap_or(2.0).max(1.0) as usize;
                        out["mediaMovel"] = Json::Array(
                            compute_moving_average(&primeira.values, periodo).into_iter().map(|v| v.map(num).unwrap_or(Json::Null)).collect(),
                        );
                    }
                }
                if let Some(aviso) = &d.aviso {
                    out["aviso"] = json!(aviso);
                }
                em_json(&out)
            }
            "dinamica_campos" => {
                // Os campos da origem (nome e tipo) numa ida só: o painel lia o
                // cabeçalho e 15 linhas de amostra célula a célula pela ponte.
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let pasta = &self.pasta;
                let campos = feathersheet_core::dinamica::campos::campos(area, |r, c| pasta.valor(i, r, c));
                em_json(&Json::Array(
                    campos
                        .iter()
                        .map(|c| json!({ "nome": c.nome, "numerico": c.numerico, "primeiroNumero": c.primeiro_numero }))
                        .collect(),
                ))
            }
            "campo_calculado_previa" => {
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let formula = a.get("formula").as_str().unwrap_or_default().to_string();
                let linhas = a.get("linhas").as_u64().unwrap_or(3) as u32;
                let pasta = &self.pasta;
                em_json(&match feathersheet_core::dinamica::campos::previa(area, &formula, linhas, |r, c| pasta.valor(i, r, c)) {
                    Ok(valores) => json!({ "valores": valores }),
                    Err(erro) => json!({ "erro": erro }),
                })
            }
            "dinamica_aplicar" => {
                // A tabela dinâmica inteira dentro do Rust: calcula lendo a aba
                // de origem viva e escreve o resultado na de destino. Antes a
                // área de origem ia ao JavaScript em texto e voltava em JSON, e
                // o resultado era escrito célula a célula pela ponte — numa
                // planilha de dois milhões de linhas, gigabytes no WebKit
                // (bug de 23/09/2026).
                let origem_id = a.u32("origemAba")?;
                let i_origem = self.pasta.indice_do_id(origem_id).ok_or_else(|| format!("não existe a aba {origem_id}"))?;
                let i_destino = self.indice(&a)?;
                let cfg = a.get("config");
                let lista = |k: &str| -> Vec<usize> {
                    cfg.get(k).and_then(Json::as_array).into_iter().flatten().filter_map(|v| v.as_u64().map(|n| n as usize)).collect()
                };
                let origem = rect_de_json(cfg.get("origem").unwrap_or(&Json::Null))?;
                let config = feathersheet_core::DinamicaConfig {
                    origem: origem.clone(),
                    linhas: lista("linhas"),
                    colunas: lista("colunas"),
                    valores: cfg
                        .get("valores")
                        .and_then(Json::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|v| {
                            Some(feathersheet_core::MedidaDinamica {
                                coluna: v.get("coluna")?.as_u64()? as usize,
                                op: feathersheet_core::AgregacaoOp::do_str(v.get("op")?.as_str()?),
                            })
                        })
                        .collect(),
                    agrupar_data: cfg.get("agrupar_data").and_then(Json::as_str).and_then(feathersheet_core::AgruparData::do_str),
                };
                let (r0, c0) = (origem.r0 as usize, origem.c0 as usize);
                let total = origem.r1.saturating_sub(origem.r0) as usize + 1;
                let pasta = &self.pasta;
                let ler = |r: usize, c: usize| pasta.valor(i_origem, (r0 + r) as u32, (c0 + c) as u32);
                // Campos calculados: colunas virtuais logo à direita da origem
                // (a `origem` já vem esticada para cobri-las), cada uma a
                // fórmula calculada linha a linha (`campo_calculado.rs`).
                let calculados: Vec<(String, String)> = cfg
                    .get("calculados")
                    .and_then(Json::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|c| Some((c.get("nome")?.as_str()?.to_string(), c.get("formula")?.as_str()?.to_string())))
                    .collect();
                let largura_real = (origem.c1.saturating_sub(origem.c0) as usize + 1).saturating_sub(calculados.len());
                let nomes: Vec<String> = (0..largura_real)
                    .map(|i| {
                        let t = feathersheet_core::celulas::values::to_text(&ler(0, i)).trim().to_string();
                        if t.is_empty() { format!("Coluna {}", i + 1) } else { t }
                    })
                    .collect();
                let exprs = calculados
                    .iter()
                    .map(|(nome, f)| {
                        feathersheet_core::dinamica::campo_calculado::compilar(f, &nomes).map_err(|e| format!("campo \"{nome}\": {e}"))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let opcoes = opcoes_da_dinamica(cfg.get("opcoes"));
                let resultado = feathersheet_core::dinamica::dinamica_layout::montar(
                    total,
                    |r, c| {
                        if c < largura_real {
                            return ler(r, c);
                        }
                        let k = c - largura_real;
                        if r == 0 {
                            return feathersheet_core::Value::Text(calculados[k].0.clone());
                        }
                        let n = feathersheet_core::dinamica::campo_calculado::avaliar(&exprs[k], &|i| {
                            feathersheet_core::dinamica::campo_calculado::numero_de(&ler(r, i))
                        });
                        feathersheet_core::Value::Number(n)
                    },
                    &config,
                    &opcoes,
                );
                let matriz = &resultado.matriz;
                let textos: Vec<Vec<String>> = matriz
                    .rows()
                    .iter()
                    .map(|linha| {
                        linha
                            .iter()
                            .map(|v| match v {
                                feathersheet_core::Value::Number(n) => feathersheet_core::celulas::number::to_js_string(*n),
                                outro => feathersheet_core::celulas::values::to_text(outro),
                            })
                            .collect()
                    })
                    .collect();
                let (altura, largura) = (textos.len(), textos.iter().map(Vec::len).max().unwrap_or(0));
                let (linha, coluna) = (a.u32("linha")?, a.u32("coluna")?);
                let limpar = a.rect("limpar").ok();
                self.pasta.editar(i_destino, |s| {
                    if let Some(area) = &limpar {
                        s.clear(area);
                    }
                    s.paste(linha, coluna, &textos, false);
                });
                let estado = self.estado();
                // O mapa de cada linha do resultado, para a grade desenhar a
                // dinâmica (tipo, nível e os botões +/−).
                let mapa: Vec<Json> = resultado.linhas.iter().map(info_para_json).collect();
                em_json(&json!({
                    "estado": estado,
                    "altura": altura,
                    "largura": largura,
                    "mapa": mapa,
                    "rotulos": resultado.colunas_de_rotulo,
                }))
            }
            "condicional_bloco" => {
                // O que a formatação condicional pinta no retângulo visível,
                // numa ida por quadro: `[[linha, coluna, formato], …]` só das
                // células que alguma regra pinta. Antes a grade perguntava ao
                // motor TypeScript célula a célula (`conditionalAt`).
                let id = a.u32("aba")?;
                let i = self.indice(&a)?;
                let (r0, c0, r1, c1) = (a.u32("r0")?, a.u32("c0")?, a.u32("r1")?, a.u32("c1")?);
                let regras = self.pasta.aba(i).expect("achada").sheet.r.clone().unwrap_or_default();
                if regras.is_empty() {
                    self.condicionais.remove(&id);
                    return em_json(&Vec::<Json>::new());
                }
                let versoes: u64 = (0..self.pasta.len()).filter_map(|k| self.pasta.aba(k)).map(|v| v.versao as u64).sum();
                let (max_row, max_col) = {
                    let s = &mut self.pasta.aba_mut(i).expect("achada").sheet;
                    (s.max_row_ts(), s.max_col_ts())
                };
                let guardada = self.condicionais.entry(id).or_insert_with(|| CondicionalGuardada {
                    regras: String::new(),
                    versoes: u64::MAX,
                    formatos: feathersheet_core::operacoes::rules::ConditionalFormats::new(),
                });
                if guardada.regras != regras {
                    guardada.formatos.set_rules(regras_de_json(&regras));
                    guardada.regras = regras;
                } else if guardada.versoes != versoes {
                    guardada.formatos.invalidate();
                }
                guardada.versoes = versoes;
                let host = HostDaRegra { pasta: &self.pasta, i, max_row, max_col };
                let mut saida = Vec::new();
                for r in r0..=r1.max(r0) {
                    for c in c0..=c1.max(c0) {
                        let valor = host.pasta.valor(i, r, c);
                        if let Some(f) = guardada.formatos.format_for(&host, r, c, &valor) {
                            saida.push(json!([r, c, f]));
                        }
                    }
                }
                em_json(&saida)
            }
            "validacao_verificar" => {
                // A pergunta que `Sheet.setCell` faria no TypeScript antes de
                // aceitar a digitação: existe uma regra de validação cobrindo
                // esta célula, e o valor digitado (ainda cru, como a pessoa
                // escreveu) passa nela? Não escreve nada — quem chama decide,
                // com a resposta, se manda `colar` (avisando, se for o caso)
                // ou recusa a entrada antes de mandar qualquer coisa.
                let i = self.indice(&a)?;
                let (r, c) = (a.u32("r")?, a.u32("c")?);
                let texto = a.get("valor").as_str().unwrap_or("").to_string();
                let va = self.pasta.aba(i).expect("achada").sheet.va.clone().unwrap_or_default();
                if va.is_empty() {
                    return em_json(&json!({ "ok": true }));
                }
                let validacoes = validacoes_de_json(&va);
                let Some(v) = validacoes.iter().find(|v| v.ranges.iter().any(|rect| rect.contains(r, c))) else {
                    return em_json(&json!({ "ok": true }));
                };
                let pasta = &self.pasta;
                let valor =
                    if texto.is_empty() { feathersheet_core::Value::Empty } else { feathersheet_core::Value::Text(texto) };
                let resultado = feathersheet_core::operacoes::validation::validar(
                    &valor,
                    &v.regra,
                    Some(|r: u32, c: u32| pasta.valor(i, r, c)),
                );
                let quando = match v.quando {
                    feathersheet_core::operacoes::validation::QuandoValidacao::Avisar => "avisar",
                    feathersheet_core::operacoes::validation::QuandoValidacao::Recusar => "recusar",
                    feathersheet_core::operacoes::validation::QuandoValidacao::Informar => "informar",
                };
                em_json(&json!({
                    "ok": resultado.ok,
                    "motivo": resultado.motivo,
                    "quando": if resultado.ok { Json::Null } else { json!(quando) },
                    "mensagem": v.mensagem,
                }))
            }
            "selecao_contar" => {
                // O rodapé: soma, contagem, mínimo e máximo numa ida só, sem
                // linha escondida. Mínimo e máximo vão nulos sem número
                // nenhum (infinito não cabe em JSON).
                let id = a.u32("aba")?;
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let sem = FaixasOcultas::default();
                let ocultas = self.ocultas.get(&id).unwrap_or(&sem);
                let pasta = &self.pasta;
                let sheet = &pasta.aba(i).expect("achada").sheet;
                let st = feathersheet_core::operacoes::ops::selection_stats_com(sheet, area, |r| !ocultas.contem(r), |r, c| pasta.valor(i, r, c));
                let finito = |x: f64| if x.is_finite() { json!(x) } else { Json::Null };
                em_json(&json!({
                    "count": st.count,
                    "numbers": st.numbers,
                    "sum": st.sum,
                    "min": finito(st.min),
                    "max": finito(st.max),
                }))
            }
            "vinculos_atualizar" => {
                // As pastas externas (`'[plan_b.plz]Aba'!A1`, `vinculo.rs`):
                // carrega o que o cálculo pediu e relê o que mudou no disco.
                // `base` é a pasta do arquivo aberto, contra a qual se
                // resolve um nome relativo.
                let base = a.texto("base").unwrap_or_default().to_string();
                let ao_vivo: HashSet<String> = a
                    .get("aoVivo")
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect();
                let mudou = self.atualizar_vinculos(&base, &ao_vivo);
                let estado = self.estado();
                // Os caminhos citados: a interface pergunta às outras janelas
                // quem tem cada um aberto.
                let caminhos: Vec<&String> = self.nomes_dos_vinculos.keys().collect();
                em_json(&json!({ "estado": estado, "mudou": mudou, "caminhos": caminhos }))
            }
            "vinculo_exportar" => {
                // O lado de quem é citado: os valores desta pasta para mandar
                // à outra janela. Com `celulas`, só essas (uma edição); sem,
                // a aba inteira (ou todas as abas) — com teto, acima dele a
                // outra janela lê do disco.
                const TETO: usize = 500_000;
                let pasta = &self.pasta;
                let lista = a.get("celulas").as_array().cloned();
                let so_aba = a.texto("aba").ok().map(str::to_lowercase);
                let mut abas = Vec::new();
                let mut total = 0usize;
                for i in 0..pasta.len() {
                    let viva = pasta.aba(i).expect("dentro do tamanho");
                    if so_aba.as_ref().is_some_and(|n| *n != viva.sheet.name.to_lowercase()) {
                        continue;
                    }
                    let chaves: Vec<(u32, u32)> = match &lista {
                        Some(l) => l
                            .iter()
                            .filter_map(|c| Some((c.get(0)?.as_u64()? as u32, c.get(1)?.as_u64()? as u32)))
                            .collect(),
                        None => viva
                            .sheet
                            .chaves_no_retangulo(&Rect::new(0, 0, u32::MAX - 1, u32::MAX - 1))
                            .into_iter()
                            .map(|k| (feathersheet_core::celulas::refs::key_row(k), feathersheet_core::celulas::refs::key_col(k)))
                            .collect(),
                    };
                    total += chaves.len();
                    if total > TETO {
                        return em_json(&json!({ "grande": true }));
                    }
                    let celulas: Vec<Json> =
                        chaves.iter().map(|&(r, c)| json!([r, c, valor_de_celula_para_json(&pasta.valor(i, r, c))])).collect();
                    abas.push(json!({ "nome": viva.sheet.name, "celulas": celulas }));
                }
                em_json(&json!({ "abas": abas }))
            }
            "vinculo_retrato" => {
                // O lado de quem cita: a pasta inteira veio da janela que a tem
                // aberta. Instala no lugar da cópia do disco.
                let arquivo = a.texto("arquivo")?.to_string();
                let Some(nome) = self.nomes_dos_vinculos.get(&arquivo).cloned() else {
                    let estado = self.estado();
                    return em_json(&json!({ "estado": estado, "mudou": false }));
                };
                let abas: Vec<feathersheet_core::planilha::sheet::Sheet> = a
                    .get("abas")
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|aba| {
                        let mut s = feathersheet_core::planilha::sheet::Sheet::new(aba.get("nome")?.as_str()?.to_string());
                        for c in aba.get("celulas")?.as_array()? {
                            let c = c.as_array()?;
                            s.set_cell_valor(c.first()?.as_u64()? as u32, c.get(1)?.as_u64()? as u32, valor_de_celula_json(c.get(2)?));
                        }
                        Some(s)
                    })
                    .collect();
                let mudou = self.pasta.instalar_externo(&nome, abas);
                let estado = self.estado();
                em_json(&json!({ "estado": estado, "mudou": mudou }))
            }
            "vinculo_valores" => {
                // Na hora da edição, a outra janela manda as células que
                // mudaram (`ui/app/vinculo-janelas.ts`): aplica na cópia dela e
                // recalcula, sem esperar o arquivo ir ao disco.
                let arquivo = a.texto("arquivo")?.to_string();
                let aba = a.texto("aba")?.to_string();
                let celulas: Vec<(u32, u32, feathersheet_core::Value)> = a
                    .get("celulas")
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|c| {
                        let c = c.as_array()?;
                        Some((c.first()?.as_u64()? as u32, c.get(1)?.as_u64()? as u32, valor_de_celula_json(c.get(2)?)))
                    })
                    .collect();
                let mudou = match self.nomes_dos_vinculos.get(&arquivo) {
                    Some(nome) => {
                        let nome = nome.clone();
                        self.pasta.atualizar_externo(&nome, &aba, &celulas)
                    }
                    None => false,
                };
                let estado = self.estado();
                em_json(&json!({ "estado": estado, "mudou": mudou }))
            }
            "tabela_total" => {
                // A linha de totais da tabela: a conta inteira numa ida, sem
                // linha escondida. A grade lia a coluna célula a célula pela
                // ponte — no fim da `planilha_2m`, 31 mil blocos e 20 s
                // (24/09/2026).
                let id = a.u32("aba")?;
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let funcao = a.texto("funcao")?.to_string();
                let sem = FaixasOcultas::default();
                let ocultas = self.ocultas.get(&id).unwrap_or(&sem);
                let pasta = &self.pasta;
                let sheet = &pasta.aba(i).expect("achada").sheet;
                // `calcularTotal` traduzido (`operacoes/tabela.rs`): a contagem
                // conta erro e não conta texto vazio, como no TypeScript.
                let total = feathersheet_core::operacoes::tabela::total_da_coluna(sheet, area, &funcao, |r| !ocultas.contem(r), |r, c| pasta.valor(i, r, c))
                    .filter(|x| x.is_finite());
                // Com `formatarEm` ({row, col}), vem também o texto pronto, no
                // formato daquela célula: a grade desenha sem formatar nada.
                match a.get("formatarEm").as_object() {
                    Some(o) => {
                        let (row, col) = (
                            o.get("row").and_then(Json::as_f64).unwrap_or(0.0).max(0.0) as u32,
                            o.get("col").and_then(Json::as_f64).unwrap_or(0.0).max(0.0) as u32,
                        );
                        let estilo = sheet.estilo_mesclado(row, col).map(|e| para_style(&e));
                        let texto = total.map(|n| feathersheet_core::planilha::bloco::texto_formatado(&feathersheet_core::Value::Number(n), estilo.as_ref()));
                        em_json(&json!({ "valor": total, "texto": texto }))
                    }
                    None => em_json(&json!(total)),
                }
            }
            "idioma" => {
                // O idioma de exibição dos números (`celulas/locale.rs`). É do
                // processo, não da pasta: vale para todas as janelas. A
                // configuração regional vem do arquivo de língua (`regional`);
                // só o código basta para as duas embutidas.
                let codigo = a.texto("idioma")?;
                let cfg = match a.get("regional") {
                    Json::Null => feathersheet_core::celulas::locale::LocaleConfig::embutida(codigo)
                        .ok_or_else(|| format!("idioma desconhecido e sem configuração regional: {codigo}"))?,
                    r => regional_de_json(codigo, r)?,
                };
                feathersheet_core::celulas::locale::set_locale_config(cfg);
                em_json(&json!({ "ok": true }))
            }
            "formatar" => {
                // `itens`: [[valor, estilo | null], …] → o texto de cada um, como
                // a célula mostraria. Para valor que não mora numa célula (colar
                // só valores, o total da tabela, a prévia do preenchimento).
                let textos: Vec<String> = a
                    .get("itens")
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|par| {
                        let valor = valor_de_celula_json(par.get(0).unwrap_or(&Json::Null));
                        let estilo = par.get(1).and_then(estilo_de_json).map(|e| para_style(&e));
                        feathersheet_core::planilha::bloco::texto_formatado(&valor, estilo.as_ref())
                    })
                    .collect();
                em_json(&textos)
            }
            "localizar" => {
                // Localizar (Ctrl+F) numa ida só: o total, a próxima depois de
                // uma posição e, se pedida, a lista com teto. A caixa refazia a
                // busca inteira no JavaScript a cada "Próxima", e lia valor,
                // estilo e texto de cada achado pela ponte.
                let i = self.indice(&a)?;
                let opcoes = opcoes_de_busca(a.get("opcoes"));
                let consulta = a.texto("consulta").unwrap_or_default();
                let pasta = &self.pasta;
                let (id, versao) = (pasta.aba(i).expect("achada").id, pasta.versao(i));
                let guardada = self.buscas.get(&id).filter(|b| b.versao == versao && b.consulta == consulta && b.opcoes == opcoes);
                if guardada.is_none() {
                    let sheet = &pasta.aba(i).expect("achada").sheet;
                    let achadas = feathersheet_core::operacoes::ops::find_all_com(sheet, consulta, opcoes, |r, c| pasta.valor(i, r, c));
                    self.buscas.insert(id, BuscaGuardada { versao, consulta: consulta.to_string(), opcoes, achadas });
                }
                let achadas = &self.buscas[&id].achadas;
                let limite = a.get("limite").as_u64().unwrap_or(0) as usize;
                let lista: Vec<[u32; 2]> = achadas.iter().take(limite).map(|m| [m.row, m.col]).collect();
                let proxima = a.get("apos").as_object().and_then(|apos| {
                    let pos = (apos.get("linha")?.as_u64()? as u32, apos.get("coluna")?.as_u64()? as u32);
                    let reverso = apos.get("reverso").and_then(Json::as_bool).unwrap_or(false);
                    proxima_achada(achadas, pos, reverso)
                });
                em_json(&json!({
                    "total": achadas.len(),
                    "achadas": lista,
                    "proxima": proxima.map(|(k, m)| json!({ "linha": m.row, "coluna": m.col, "indice": k })),
                }))
            }
            "substituir" => {
                // Substituir tudo, ou só a célula pedida (o botão
                // "Substituir"), numa transação: um Ctrl+Z desfaz.
                let i = self.indice(&a)?;
                let opcoes = opcoes_de_busca(a.get("opcoes"));
                let consulta = a.texto("consulta").unwrap_or_default().to_string();
                let troca = a.texto("troca").unwrap_or_default().to_string();
                let celula = a.get("celula").as_object().and_then(|c| Some((c.get("linha")?.as_u64()? as u32, c.get("coluna")?.as_u64()? as u32)));
                let achadas: Vec<_> = {
                    let pasta = &self.pasta;
                    let sheet = &pasta.aba(i).expect("achada").sheet;
                    feathersheet_core::operacoes::ops::find_all_com(sheet, &consulta, opcoes, |r, c| pasta.valor(i, r, c))
                        .into_iter()
                        .filter(|m| celula.is_none_or(|(r, c)| (m.row, m.col) == (r, c)))
                        .collect()
                };
                let mut total = 0;
                if !achadas.is_empty() {
                    self.pasta.editar(i, |s| total = feathersheet_core::operacoes::ops::replace_all_em(s, &achadas, &consulta, &troca, opcoes));
                }
                let estado = self.estado();
                em_json(&json!({ "estado": estado, "total": total }))
            }
            "estatisticas_coluna" => {
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let max_row = self.pasta.aba_mut(i).expect("achada").sheet.max_row_ts();
                let pasta = &self.pasta;
                let st = feathersheet_core::operacoes::ops::column_stats_com(area, max_row, |r, c| pasta.valor(i, r, c));
                em_json(&json!({
                    "count": st.count, "numbers": st.numbers, "empty": st.empty, "distinct": st.distinct,
                    "sum": st.sum, "average": st.average, "min": st.min, "max": st.max,
                }))
            }
            "perfil_coluna" => {
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let max_row = self.pasta.aba_mut(i).expect("achada").sheet.max_row_ts();
                let pasta = &self.pasta;
                let p = feathersheet_core::operacoes::perfil::perfil_coluna(area, max_row, |r, c| pasta.valor(i, r, c));
                em_json(&json!({
                    "linhas": p.linhas, "vazias": p.vazias, "numeros": p.numeros, "textos": p.textos,
                    "logicos": p.logicos, "erros": p.erros, "distintos": p.distintos, "repetidos": p.repetidos,
                    "min": p.min, "max": p.max, "media": p.media,
                    "quartis": p.quartis.map(|q| json!({
                        "q1": q.q1, "mediana": q.mediana, "q3": q.q3,
                        "limiteBaixo": q.limite_baixo, "limiteAlto": q.limite_alto,
                    })),
                    "histograma": p.histograma.iter().map(|f| json!({ "de": f.de, "ate": f.ate, "qtd": f.qtd })).collect::<Vec<_>>(),
                    "outliers": p.outliers,
                    "exemplosOutliers": p.exemplos_outliers.iter().map(|&(linha, valor)| json!({ "linha": linha, "valor": valor })).collect::<Vec<_>>(),
                    "frequentes": p.frequentes.iter().map(|(valor, qtd)| json!({ "valor": valor, "qtd": qtd })).collect::<Vec<_>>(),
                }))
            }
            "regiao_atual" => {
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let r = feathersheet_core::operacoes::ops::regiao_atual(&self.pasta.aba(i).expect("achada").sheet, area);
                em_json(&json!({ "r0": r.r0, "c0": r.c0, "r1": r.r1, "c1": r.c1 }))
            }
            "explicar_erro" => {
                // Por que a célula deu erro (`formula/explicar.rs`). `null`
                // quando ela não tem erro.
                let i = self.indice(&a)?;
                let e = feathersheet_core::formula::explicar::explicar_erro(&self.pasta, i, a.u32("linha")?, a.u32("coluna")?);
                em_json(&e.map(|e| json!({
                    "code": e.code,
                    "meaning": e.meaning,
                    "summary": e.summary,
                    "hint": e.hint,
                    "steps": e.steps.iter().map(|p| {
                        let mut passo = json!({ "cell": p.cell, "formula": p.formula, "culprit": p.culprit });
                        if let Some(o) = &p.onde { passo["where"] = json!(o); }
                        if let Some(c) = &p.blame_cell { passo["blameCell"] = json!(c); }
                        passo
                    }).collect::<Vec<_>>(),
                })))
            }
            "tabela_sugerida" => {
                // O Ctrl+T: a região em volta da célula e se a primeira linha
                // parece cabeçalho. `null` quando a célula está vazia.
                let i = self.indice(&a)?;
                let (linha, coluna) = (a.u32("linha")?, a.u32("coluna")?);
                if !self.pasta.aba(i).expect("achada").sheet.has_content(linha, coluna) {
                    return em_json(&Json::Null);
                }
                let r = self.sheet_mut(&a)?.regiao_da_tabela(linha, coluna);
                let pasta = &self.pasta;
                let cabecalho = feathersheet_core::operacoes::ops::parece_cabecalho(r.clone(), |row, col| pasta.valor(i, row, col));
                let mut resposta = json!({ "area": rect_para_json(&r), "temCabecalho": cabecalho });
                // Com `usados` (as tabelas da aba e os nomes da pasta), vem
                // também o nome livre da série `Tabela1`, `Tabela2`…
                if let Some(usados) = a.get("usados").as_array() {
                    let usados: Vec<String> = usados.iter().filter_map(|n| n.as_str().map(str::to_string)).collect();
                    resposta["nome"] = json!(feathersheet_core::operacoes::tabela::nome_disponivel(&usados, "Tabela"));
                }
                em_json(&resposta)
            }
            "tabela_crescer" => {
                // Escreveu logo abaixo (ou à direita) de uma tabela: ela
                // cresce. As tabelas moram na `Aba` do TypeScript e vêm só
                // com o que a conta precisa. Responde `{indice, area}` ou nulo.
                let tabelas: Vec<feathersheet_core::operacoes::tabela::AreaDaTabela> = a
                    .get("tabelas")
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|t| feathersheet_core::operacoes::tabela::AreaDaTabela {
                        area: rect_de_json(t.get("area").unwrap_or(&Json::Null)).unwrap_or_else(|_| Rect::new(u32::MAX, u32::MAX, u32::MAX, u32::MAX)),
                        mostrar_totais: t.get("mostrarTotais").and_then(Json::as_bool).unwrap_or(false),
                        so_filtro: t.get("soFiltro").and_then(Json::as_bool).unwrap_or(false),
                    })
                    .collect();
                let cresceu = feathersheet_core::operacoes::tabela::crescer_tabela(&tabelas, a.u32("linha")?, a.u32("coluna")?);
                em_json(&cresceu.map(|(indice, area)| json!({ "indice": indice, "area": rect_para_json(&area) })))
            }
            "regiao_da_tabela" => {
                let (linha, coluna) = (a.u32("linha")?, a.u32("coluna")?);
                let r = self.sheet_mut(&a)?.regiao_da_tabela(linha, coluna);
                em_json(&json!({ "r0": r.r0, "c0": r.c0, "r1": r.r1, "c1": r.c1 }))
            }
            "autosoma" => {
                let i = self.indice(&a)?;
                let (linha, coluna) = (a.u32("linha")?, a.u32("coluna")?);
                let pasta = &self.pasta;
                em_json(&feathersheet_core::operacoes::ops::formula_da_autosoma(linha, coluna, |r, c| pasta.valor(i, r, c)))
            }
            "preencher" => {
                // A alça de preenchimento inteira no Rust: planeja e escreve.
                let i = self.indice(&a)?;
                let (origem, destino) = (a.rect("origem")?, a.rect("destino")?);
                let mut total = 0;
                self.pasta.editar(i, |s| total = s.preencher(&origem, &destino));
                let estado = self.estado();
                em_json(&json!({ "estado": estado, "total": total }))
            }
            "preencher_ate_o_fim" => {
                // O duplo clique na alça: acha até onde descer e preenche, numa
                // ida. `planejar` só responde o destino (a interface confere a
                // proteção antes de escrever). `destino` nulo: nada a fazer.
                let i = self.indice(&a)?;
                let selecao = a.rect("selecao")?;
                let colunas = a.u32("colunas").unwrap_or(feathersheet_core::celulas::refs::MAX_COLS);
                let planejar = a.get("planejar").as_bool().unwrap_or(false);
                let mut feito = None;
                if planejar {
                    feito = self.sheet_mut(&a)?.preencher_ate_o_fim(&selecao, colunas, true);
                } else {
                    self.pasta.editar(i, |s| feito = s.preencher_ate_o_fim(&selecao, colunas, false));
                }
                let estado = self.estado();
                em_json(&json!({
                    "estado": estado,
                    "destino": feito.as_ref().map(|(r, _)| rect_para_json(r)),
                    "total": feito.map_or(0, |(_, n)| n),
                }))
            }
            // Arrastar o bloco pela borda (24/09/2026, só no Rust): conteúdo,
            // estilo e as fórmulas da pasta que apontavam para ele.
            "mover_intervalo" => {
                let i = self.indice(&a)?;
                let origem = a.rect("origem")?;
                let d = |k: &str| a.get(k).as_i64().map(|v| v as i32).ok_or_else(|| format!("falta \"{k}\""));
                let (dr, dc) = (d("linhas")?, d("colunas")?);
                // Sem `substituir`, destino com conteúdo (fora da própria
                // origem) não é tocado: a interface pergunta antes, como o Excel.
                if !a.get("substituir").as_bool().unwrap_or(false) {
                    let s = &self.pasta.aba(i).ok_or("aba sumiu")?.sheet;
                    let destino = feathersheet_core::planilha::mover::Movimento { aba: String::new(), origem: origem.normalized(), dr, dc }
                        .destino()
                        .ok_or("o destino sai da planilha")?;
                    let o = origem.normalized();
                    let ocupado = s.chaves_no_retangulo(&destino).into_iter().any(|k| {
                        let (r, c) = (feathersheet_core::celulas::refs::key_row(k), feathersheet_core::celulas::refs::key_col(k));
                        let na_origem = r >= o.r0 && r <= o.r1 && c >= o.c0 && c <= o.c1;
                        !na_origem && s.has_content(r, c)
                    });
                    if ocupado {
                        let estado = self.estado();
                        return em_json(&json!({ "estado": estado, "ocupado": true, "movido": false }));
                    }
                }
                let movido = self.pasta.mover_intervalo(i, origem, dr, dc).is_some();
                let estado = self.estado();
                em_json(&json!({ "estado": estado, "movido": movido }))
            }
            // Passo 3 (23/09/2026): as operações por linha do `ops.ts`. Quem
            // depende do valor calculado lê pela pasta (uma fórmula que cita
            // outra aba) e só então escreve; cada uma é um Ctrl+Z só.
            "ordenar" => {
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let cabecalho = a.get("cabecalho").as_bool().unwrap_or(false);
                let chaves: Vec<feathersheet_core::operacoes::ops::SortKey> = a
                    .get("chaves")
                    .as_array()
                    .map(|v| v.as_slice())
                    .unwrap_or(&[])
                    .iter()
                    .filter_map(|k| {
                        Some(feathersheet_core::operacoes::ops::SortKey {
                            col: k.get("coluna")?.as_u64()? as u32,
                            ascending: k.get("crescente").and_then(Json::as_bool).unwrap_or(true),
                        })
                    })
                    .collect();
                let max_row = self.pasta.aba_mut(i).ok_or("aba inexistente")?.sheet.max_row_ts();
                let plano = {
                    let pasta = &self.pasta;
                    feathersheet_core::operacoes::ops::planejar_ordenacao(area.clone(), &chaves, cabecalho, max_row, |r, c| pasta.valor(i, r, c))
                };
                if let Some(p) = &plano {
                    self.pasta.editar(i, |s| feathersheet_core::operacoes::ops::aplicar_ordenacao(s, area.c0, area.c1, p));
                }
                let estado = self.estado();
                em_json(&json!({ "estado": estado, "total": plano.map_or(0, |p| p.ordem.len()) }))
            }
            "remover_repetidas" => {
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let cabecalho = a.get("cabecalho").as_bool().unwrap_or(false);
                let colunas: Option<Vec<u32>> =
                    a.get("colunas").as_array().map(|v| v.iter().filter_map(|c| Some(c.as_u64()? as u32)).collect());
                let max_row = self.pasta.aba_mut(i).ok_or("aba inexistente")?.sheet.max_row_ts();
                let plano = {
                    let pasta = &self.pasta;
                    feathersheet_core::operacoes::ops::planejar_repetidas(area.clone(), cabecalho, colunas.as_deref(), max_row, |r, c| pasta.valor(i, r, c))
                };
                if let Some(p) = &plano {
                    self.pasta.editar(i, |s| feathersheet_core::operacoes::ops::aplicar_repetidas(s, area.c0, area.c1, p));
                }
                let estado = self.estado();
                em_json(&json!({ "estado": estado, "total": plano.map_or(0, |p| p.removidas) }))
            }
            "dividir_texto_prever" => {
                // Só a leitura (planejar_divisao): o assistente mostra a
                // prévia antes de a pessoa escolher o tipo de cada coluna e
                // confirmar — nada é escrito aqui.
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let opcoes = opcoes_de_divisao(&a);
                let limite = a.get("limite").as_u64().unwrap_or(20) as usize;
                let max_row = self.pasta.aba_mut(i).ok_or("aba inexistente")?.sheet.max_row_ts();
                let linhas = {
                    let pasta = &self.pasta;
                    feathersheet_core::operacoes::ops::planejar_divisao(area.clone(), &opcoes, max_row, |r, c| pasta.valor(i, r, c))
                };
                let colunas = linhas.as_ref().map_or(0, |l| l.iter().map(|(_, p)| p.len()).max().unwrap_or(0));
                let amostra: Vec<&Vec<String>> = linhas.as_ref().map_or(Vec::new(), |l| l.iter().take(limite).map(|(_, p)| p).collect());
                em_json(&json!({ "colunas": colunas, "amostra": amostra }))
            }
            "dividir_texto" => {
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let opcoes = opcoes_de_divisao(&a);
                let tipos = tipos_de_divisao(a.get("tipos"));
                let max_row = self.pasta.aba_mut(i).ok_or("aba inexistente")?.sheet.max_row_ts();
                let linhas = {
                    let pasta = &self.pasta;
                    feathersheet_core::operacoes::ops::planejar_divisao(area.clone(), &opcoes, max_row, |r, c| pasta.valor(i, r, c))
                };
                let mut total = 0;
                if let Some(l) = &linhas {
                    self.pasta.editar(i, |s| total = feathersheet_core::operacoes::ops::aplicar_divisao(s, area.c0, l, tipos.as_deref()));
                }
                let estado = self.estado();
                em_json(&json!({ "estado": estado, "total": total }))
            }
            "aparar_espacos" => {
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let mut total = 0;
                self.pasta.editar(i, |s| total = feathersheet_core::operacoes::ops::trim_spaces(s, area));
                let estado = self.estado();
                em_json(&json!({ "estado": estado, "total": total }))
            }
            "bordas" => {
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let tipo = a.texto("tipo")?.to_string();
                let cor = a.get("cor").as_str().map(str::to_string);
                self.pasta.editar(i, |s| feathersheet_core::operacoes::ops::apply_borders(s, area, &tipo, cor.as_deref()));
                self.responder_estado()
            }
            "casas_decimais" => {
                let i = self.indice(&a)?;
                let area = a.rect("area")?;
                let passo = a.get("passo").as_i64().unwrap_or(0) as i32;
                self.pasta.editar(i, |s| feathersheet_core::operacoes::ops::step_decimals(s, area, passo));
                self.responder_estado()
            }
            "celulas" => {
                let i = self.indice(&a)?;
                Ok(feathersheet_core::planilha::bloco::montar_celulas(&self.pasta, i))
            }
            "estilos" => {
                let i = self.indice(&a)?;
                let s = &self.pasta.aba(i).expect("achada").sheet;
                let lista: Vec<Json> = a
                    .get("indices")
                    .as_array()
                    .map(|v| v.as_slice())
                    .unwrap_or(&[])
                    .iter()
                    .map(|x| {
                        x.as_u64()
                            .and_then(|i| s.style_parts.get(i as usize))
                            .map(estilo_para_json)
                            .unwrap_or_else(|| json!({}))
                    })
                    .collect();
                em_json(&lista)
            }
            "estilos_gerais" => {
                let i = self.indice(&a)?;
                let s = &self.pasta.aba(i).expect("achada").sheet;
                let mut colunas: Vec<(u32, Json)> =
                    s.col_styles.iter().map(|(c, e)| (*c, estilo_para_json(e))).collect();
                let mut linhas: Vec<(u32, Json)> =
                    s.row_styles.iter().map(|(r, e)| (*r, estilo_para_json(e))).collect();
                colunas.sort_by_key(|p| p.0);
                linhas.sort_by_key(|p| p.0);
                em_json(&json!({
                    "colunas": colunas,
                    "linhas": linhas,
                    "aba": s.sheet_style.as_ref().map(estilo_para_json),
                }))
            }
            "proximo_desfazer" => {
                let i = self.indice(&a)?;
                let s = &self.pasta.aba(i).expect("achada").sheet;
                em_json(&s.celulas_do_proximo(a.logico("refazer")))
            }

            // ------------------------------------------------------ escrita
            "celula_gravar" => {
                // O funil único de escrita de UMA célula (`Sheet.setCell` do
                // TypeScript) — o caminho de digitar. A guarda de validação
                // mora aqui, não em `set_cell`: `sheet-core` não lê JSON
                // (regra 11), então quem entende o `va` opaco é a ponte.
                let (row, col) = (a.u32("linha")?, a.u32("coluna")?);
                let raw = a.texto("raw").unwrap_or_default().to_string();
                self.recusar_se_invalido(&a, row, col, &raw)?;
                self.editar(&a, |s| s.set_cell(row, col, &raw))
            }
            "celula_aplicar" => {
                let (row, col) = (a.u32("linha")?, a.u32("coluna")?);
                let snap = snap_de_json(a.get("snap"));
                let gravar = a.logico("gravar");
                self.editar(&a, |s| {
                    s.apply(row, col, snap, gravar);
                })
            }
            "celulas_aplicar" => {
                // A carga: [[linha, coluna, raw, estilo|null], …], sem desfazer.
                let lista = a.get("celulas").as_array().cloned().unwrap_or_default();
                self.editar(&a, |s| {
                    for c in &lista {
                        let row = c.get(0).and_then(Json::as_u64).unwrap_or(0) as u32;
                        let col = c.get(1).and_then(Json::as_u64).unwrap_or(0) as u32;
                        let raw = c.get(2).and_then(Json::as_str).unwrap_or_default().to_string();
                        let style = c.get(3).and_then(estilo_de_json).filter(|e| !e.is_empty());
                        let snap = if raw.is_empty() && style.is_none() { None } else { Some(Snap { raw, style }) };
                        s.apply(row, col, snap, false);
                    }
                })
            }
            "estilo_aplicar" => {
                let area = a.rect("area")?;
                let patch = estilo_de_json(a.get("patch")).unwrap_or_default();
                self.editar(&a, |s| s.set_style(&area, &patch))
            }
            "estilos_gerais_definir" => {
                let colunas = a.get("colunas").as_array().cloned().unwrap_or_default();
                let linhas = a.get("linhas").as_array().cloned().unwrap_or_default();
                let da_aba = estilo_de_json(a.get("aba")).filter(|e| !e.is_empty());
                let par = |v: &Json| -> Option<(u32, EstiloParcial)> {
                    Some((v.get(0)?.as_u64()? as u32, estilo_de_json(v.get(1)?)?))
                };
                self.editar(&a, |s| {
                    s.col_styles = colunas.iter().filter_map(par).collect();
                    s.row_styles = linhas.iter().filter_map(par).collect();
                    s.sheet_style = da_aba;
                })
            }
            "limpar" => {
                let area = a.rect("area")?;
                self.editar(&a, |s| s.clear(&area))
            }
            "colar" => {
                let (row, col) = (a.u32("linha")?, a.u32("coluna")?);
                let desfazer = a.0.get("desfazer").and_then(Json::as_bool).unwrap_or(true);
                let linhas: Vec<Vec<String>> = a
                    .get("linhas")
                    .as_array()
                    .map(|l| {
                        l.iter()
                            .map(|linha| {
                                linha
                                    .as_array()
                                    .map(|c| c.iter().map(|x| x.as_str().unwrap_or_default().to_string()).collect())
                                    .unwrap_or_default()
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                // Guarda de validação no funil único de escrita: só a
                // digitação de UMA célula (o caminho de `startEdit`/Enter,
                // aqui um `colar` 1×1) é barrada — colar em lote não passa
                // por este freio, para não escolher célula a célula dentro
                // de uma transação só. A checagem "avisar" fica com
                // `validacao_verificar`, que a interface chama antes de
                // digitar; aqui é só o freio de segurança para "recusar".
                if linhas.len() == 1 && linhas.first().is_some_and(|l| l.len() == 1) {
                    self.recusar_se_invalido(&a, row, col, &linhas[0][0])?;
                }
                self.editar(&a, |s| s.paste(row, col, &linhas, desfazer))
            }
            "transacao" => {
                let acao = a.texto("acao")?.to_string();
                match acao.as_str() {
                    "abrir" => {
                        self.sheet_mut(&a)?.abrir_transacao();
                        self.responder_estado()
                    }
                    "fechar" => {
                        self.sheet_mut(&a)?.fechar_transacao();
                        self.responder_estado()
                    }
                    "reverter" => self.editar(&a, |s| s.reverter_transacao()),
                    outra => Err(format!("transação não sabe \"{outra}\"")),
                }
            }
            "desfazer" => self.editar(&a, |s| {
                s.undo();
            }),
            "refazer" => self.editar(&a, |s| {
                s.redo();
            }),
            "historico_truncar" => {
                let n = a.u32("profundidade")? as usize;
                self.editar(&a, |s| s.truncar_historico(n))
            }
            "historico_limpar" => {
                self.sheet_mut(&a)?.clear_history();
                self.responder_estado()
            }
            "estrutura_mover" => {
                // Inserir/excluir linha ou coluna, com o conserto do texto
                // das fórmulas desta aba e das outras (ver
                // `Workbook::mover_estrutura`).
                let i = self.indice(&a)?;
                let coluna = a.logico("coluna");
                let (em, n) = (a.u32("em")?, a.u32("quantas")?);
                let remover = a.logico("remover");
                self.pasta.mover_estrutura(i, coluna, em, n, remover);
                // Os metadados que a interface mandou (tabelas, regras,
                // mesclas…) voltam acertados junto com o estado.
                let mut meta = a.get("metadados").clone();
                if meta.is_object() {
                    let aba = self.pasta.aba(i).expect("achada").sheet.name.clone();
                    metadados::mover(&mut meta, &metadados::Movimento { coluna, em, quantas: n, removendo: remover, aba: &aba });
                }
                let estado = self.estado();
                em_json(&json!({ "estado": estado, "metadados": meta }))
            }
            outro => Err(format!("comando desconhecido: \"{outro}\"")),
        }
    }
}

// --------------------------------------------------------------- filtro

/// Um valor da planilha como JSON: número, texto, lógico, `null` (vazio); o
/// erro vai pelo código (`#N/D`).
fn valor_para_json(v: &feathersheet_core::Value) -> Json {
    use feathersheet_core::Value;
    match v {
        Value::Empty => Json::Null,
        Value::Number(n) if n.is_finite() => json!(n),
        Value::Bool(b) => json!(b),
        Value::Text(t) => json!(t),
        outro => json!(feathersheet_core::operacoes::filtro::texto_js(outro)),
    }
}

fn valor_de_json(v: &Json) -> feathersheet_core::Value {
    use feathersheet_core::Value;
    match v {
        Json::Null => Value::Empty,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => n.as_f64().map(Value::Number).unwrap_or(Value::Empty),
        Json::String(t) => Value::Text(t.clone()),
        outro => Value::Text(outro.to_string()),
    }
}

/// O critério como a `PlanilhaRust` manda: `{ lista }`, `{ foraDaLista }`,
/// `{ op, val }` ou `{ classico }`.
fn criterio_de_json(f: &Json) -> Option<feathersheet_core::operacoes::filtro::Criterio> {
    use feathersheet_core::operacoes::filtro::Criterio;
    let lista = |k: &str| f.get(k).and_then(Json::as_array).map(|a| a.iter().map(valor_de_json).collect());
    if let Some(l) = lista("lista") {
        return Some(Criterio::Lista(l));
    }
    if let Some(l) = lista("foraDaLista") {
        return Some(Criterio::ForaDaLista(l));
    }
    if let Some(op) = f.get("op").and_then(Json::as_str) {
        return Some(Criterio::Regra { op: op.to_string(), val: valor_de_json(f.get("val").unwrap_or(&Json::Null)) });
    }
    f.get("classico").map(|c| Criterio::Classico(valor_de_json(c)))
}

/// A configuração regional do bloco `idioma` do arquivo de língua
/// (`src/idiomas/<código>.json`), no formato que o TypeScript manda.
fn regional_de_json(codigo: &str, r: &Json) -> Result<feathersheet_core::celulas::locale::LocaleConfig, String> {
    let texto = |k: &str| r.get(k).and_then(Json::as_str).map(str::to_string).ok_or_else(|| format!("idioma {codigo}: falta \"{k}\""));
    let letra = |k: &str| -> Result<char, String> {
        let t = texto(k)?;
        let mut cs = t.chars();
        match (cs.next(), cs.next()) {
            (Some(c), None) => Ok(c),
            _ => Err(format!("idioma {codigo}: \"{k}\" tem de ser um caractere só, veio \"{t}\"")),
        }
    };
    let sim = |k: &str, padrao: bool| r.get(k).and_then(Json::as_bool).unwrap_or(padrao);
    Ok(feathersheet_core::celulas::locale::LocaleConfig {
        code: codigo.to_string(),
        name: texto("nome")?,
        decimal_separator: letra("decimal")?,
        thousands_separator: letra("milhar")?,
        formula_arg_separator: letra("separadorDeArgumentos")?,
        currency_symbol: texto("moeda")?,
        currency_code: texto("codigoDaMoeda")?,
        currency_before: sim("moedaAntes", true),
        currency_space: sim("espacoNaMoeda", true),
    })
}

// ------------------------------------------------------------ metadados

/// Os metadados da aba no formato do FSH2 (`tc`, `w`, `h`, `g`, `pr`, `r`,
/// `tb`, `oq`, `dn`). O Rust não entende gráfico nem regra ainda: guarda o
/// JSON como veio e o devolve igual.
fn metadados_para_json(s: &Sheet) -> Json {
    let opaco = |v: &Option<String>| -> Json {
        v.as_ref().and_then(|t| serde_json::from_str(t).ok()).unwrap_or(Json::Null)
    };
    let mut w: Vec<[u32; 2]> = s.col_widths.iter().map(|(&c, &w)| [c, w]).collect();
    let mut h: Vec<[u32; 2]> = s.row_heights.iter().map(|(&r, &h)| [r, h]).collect();
    w.sort_unstable();
    h.sort_unstable();
    json!({
        "tc": s.tab_color,
        "w": w,
        "h": h,
        "g": opaco(&s.g),
        "pr": opaco(&s.pr),
        "r": opaco(&s.r),
        "tb": opaco(&s.tb),
        "oq": opaco(&s.oq),
        "dn": opaco(&s.dn),
        "mc": opaco(&s.mc),
        "im": opaco(&s.im),
        "va": opaco(&s.va),
        "bt": opaco(&s.bt),
        "tn": opaco(&s.tn),
        "ob": opaco(&s.ob),
        "ce": opaco(&s.ce),
    })
}

/// O contrário: **todo** campo é regravado, e o que falta (ou vem `null`)
/// apaga. Só gravar o que veio deixaria o último gráfico apagado voltar no
/// arquivo.
fn definir_metadados(s: &mut Sheet, m: &Json) {
    let opaco = |k: &str| -> Option<String> {
        match m.get(k) {
            None | Some(Json::Null) => None,
            Some(Json::Array(a)) if a.is_empty() => None,
            Some(v) => Some(v.to_string()),
        }
    };
    let pares = |k: &str| -> std::collections::HashMap<u32, u32> {
        m.get(k)
            .and_then(Json::as_array)
            .map(|lista| {
                lista
                    .iter()
                    .filter_map(|p| {
                        let a = p.get(0)?.as_f64()?;
                        let b = p.get(1)?.as_f64()?;
                        Some((a.max(0.0) as u32, b.max(0.0).round() as u32))
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    s.tab_color = m.get("tc").and_then(Json::as_str).map(str::to_string);
    s.col_widths = pares("w");
    s.row_heights = pares("h");
    s.g = opaco("g");
    s.pr = opaco("pr");
    s.r = opaco("r");
    s.tb = opaco("tb");
    s.oq = opaco("oq");
    s.dn = opaco("dn");
    s.mc = opaco("mc");
    s.im = opaco("im");
    s.va = opaco("va");
    s.bt = opaco("bt");
    s.tn = opaco("tn");
    s.ob = opaco("ob");
    s.ce = opaco("ce");
}

/// As opções de layout da dinâmica, do JSON do painel. Ausentes = as da
/// dinâmica antiga (tabular, rótulo repetido, sem subtotal).
fn opcoes_da_dinamica(j: Option<&Json>) -> feathersheet_core::dinamica::dinamica_layout::Opcoes {
    use feathersheet_core::dinamica::dinamica_layout::{Layout, Opcoes};
    let mut o = Opcoes::antigas();
    let Some(j) = j else { return o };
    if j.get("layout").and_then(Json::as_str) == Some("compacto") {
        o.layout = Layout::Compacto;
    }
    if let Some(b) = j.get("repetir").and_then(Json::as_bool) {
        o.repetir_rotulos = b;
    }
    if let Some(b) = j.get("subtotais").and_then(Json::as_bool) {
        o.subtotais = b;
    }
    o.recolhidos = j
        .get("recolhidos")
        .and_then(Json::as_array)
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    o.filtros = j
        .get("filtros")
        .and_then(Json::as_array)
        .into_iter()
        .flatten()
        .filter_map(|f| {
            let coluna = f.get("coluna")?.as_u64()? as usize;
            let valores = f.get("valores")?.as_array()?.iter().filter_map(|v| v.as_str().map(str::to_string)).collect();
            let excluir = f.get("excluir").and_then(Json::as_bool).unwrap_or(false);
            Some(feathersheet_core::dinamica::dinamica_layout::Filtro { coluna, valores, excluir })
        })
        .collect();
    o
}

/// `[tipo, nível, [[coluna, caminho, recolhido], …]]`, com o tipo numa
/// letra: `c`abeçalho, `g`rupo, `i`tem, `s`ubtotal, `t`otal.
fn info_para_json(info: &feathersheet_core::dinamica::dinamica_layout::InfoLinha) -> Json {
    use feathersheet_core::dinamica::dinamica_layout::TipoLinha;
    let tipo = match info.tipo {
        TipoLinha::Cabecalho => "c",
        TipoLinha::Grupo => "g",
        TipoLinha::Item => "i",
        TipoLinha::Subtotal => "s",
        TipoLinha::Total => "t",
    };
    let marcas: Vec<Json> = info.marcas.iter().map(|m| json!([m.coluna, m.caminho, m.recolhido])).collect();
    json!([tipo, info.nivel, marcas])
}

/// A resposta pronta de uma tarefa (a função de macro): uma lista de listas
/// derrama como tabela, o resto é um valor (`feathersheet_io::pendentes`).
fn resposta_de_valor(v: &Json) -> feathersheet_core::planilha::assincrono::Resposta {
    use feathersheet_core::planilha::assincrono::Resposta;
    match v.as_array() {
        Some(linhas) if !linhas.is_empty() && linhas.iter().all(Json::is_array) => Resposta::Matriz(
            linhas
                .iter()
                .map(|l| l.as_array().into_iter().flatten().map(feathersheet_io::pendentes::valor_de_json).collect())
                .collect(),
        ),
        _ => Resposta::Valor(feathersheet_io::pendentes::valor_de_json(v)),
    }
}

/// O contrário do [`valor_de_celula_json`]: o valor de uma célula para
/// mandar à outra janela.
fn valor_de_celula_para_json(v: &feathersheet_core::Value) -> Json {
    use feathersheet_core::Value;
    match v {
        Value::Number(n) if n.is_finite() => json!(n),
        Value::Text(t) => json!(t),
        Value::Bool(b) => json!(b),
        Value::Error(e) => json!({ "erro": e.code.as_str() }),
        _ => Json::Null,
    }
}

/// Um valor de célula vindo da outra janela: número, texto, lógico, nulo
/// (vazio) ou `{ "erro": "#DIV/0!" }`. O `valor_de_json` do filtro trata
/// objeto como texto, e erro de célula precisa voltar a ser erro.
fn valor_de_celula_json(v: &Json) -> feathersheet_core::Value {
    use feathersheet_core::Value;
    match v {
        Json::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
        Json::String(t) => Value::Text(t.clone()),
        Json::Bool(b) => Value::Bool(*b),
        Json::Object(o) => match o.get("erro").and_then(Json::as_str).and_then(feathersheet_core::celulas::values::ErrorCode::from_code) {
            Some(code) => Value::Error(feathersheet_core::celulas::values::CellError::new(code)),
            None => Value::Empty,
        },
        _ => Value::Empty,
    }
}


/// Uma tabela do OpenQuery como a janela a manda: `cabecalho` e `linhas`,
/// com os valores no JSON do JavaScript (erro como `{ "erro": "#N/D" }`).
fn tabela_de_json(j: &Json) -> feathersheet_core::Tabela {
    feathersheet_core::Tabela {
        cabecalho: j
            .get("cabecalho")
            .and_then(Json::as_array)
            .map(|c| c.iter().map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())).collect())
            .unwrap_or_default(),
        linhas: j
            .get("linhas")
            .and_then(Json::as_array)
            .map(|ls| {
                ls.iter()
                    .map(|l| l.as_array().map(|vs| vs.iter().map(valor_de_celula_json).collect()).unwrap_or_default())
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Um passo do OpenQuery vindo do JavaScript. Os campos que são valor de
/// célula (`criterio`, `de`, `para`) chegam no JSON do JavaScript e são
/// trocados pelo formato do `Value` antes do serde ler o resto.
fn passo_de_json(p: &Json) -> Result<feathersheet_core::Passo, String> {
    let mut p = p.clone();
    if let Some(o) = p.as_object_mut() {
        for campo in ["criterio", "de", "para"] {
            // `tipar` também tem `para`, mas é o nome do tipo, texto puro.
            if campo == "para" && o.get("tipo").and_then(Json::as_str) == Some("tipar") {
                continue;
            }
            if let Some(v) = o.get(campo) {
                let valor = valor_de_celula_json(v);
                o.insert(campo.into(), serde_json::to_value(valor).map_err(|e| e.to_string())?);
            }
        }
    }
    serde_json::from_value(p).map_err(|e| format!("passo de consulta inválido: {e}"))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn chamar(p: &mut Ponte, cmd: &str, corpo: Json) -> Json {
        let bytes = p.chamar(cmd, &serde_json::to_vec(&corpo).unwrap()).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// A importação em lotes (o aplicativo solta a ponte entre um e outro)
    /// acerta a aba pelo `id` mesmo se a pessoa mexeu nas abas no meio.
    #[test]
    fn csv_em_lotes_acha_a_aba_pelo_id_mesmo_com_abas_mexidas() {
        let mut p = Ponte::default();
        let id = p.csv_comecar("CSV".into());
        let linha = |a: &str, b: &str| vec![a.to_string(), b.to_string()];
        p.csv_colar_lote(id, 0, &[linha("nome", "valor"), linha("ana", "10")]);
        // Entre os lotes: a primeira aba some e outra aparece no fim.
        assert!(p.pasta.remover_aba(0));
        p.pasta.criar_aba("Outra");
        p.csv_colar_lote(id, 2, &[linha("bia", "20")]);
        let r: Json = serde_json::from_slice(&p.csv_concluir(id, 3).unwrap()).unwrap();
        assert_eq!(r["linhas"], 3);
        let i = p.pasta.indice_do_id(id).unwrap();
        let s = &p.pasta.aba(i).unwrap().sheet;
        assert_eq!(s.get_raw(0, 0), "nome");
        assert_eq!(s.get_raw(2, 0), "bia");
        assert_eq!(s.get_value(2, 1), feathersheet_core::celulas::values::Value::Number(20.0));
    }

    #[test]
    fn gravar_sobe_a_versao_e_desfazer_volta() {
        let mut p = Ponte::default();
        let e0 = chamar(&mut p, "pasta_estado", json!({}));
        let id = e0["abas"][0]["id"].as_u64().unwrap();
        let v0 = e0["abas"][0]["versao"].as_u64().unwrap();
        let e1 = chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 2, "coluna": 3, "raw": "7" }));
        assert!(e1["abas"][0]["versao"].as_u64().unwrap() > v0);
        assert_eq!(e1["abas"][0]["maxRow"], 2);
        assert_eq!(e1["abas"][0]["podeDesfazer"], true);
        let e2 = chamar(&mut p, "desfazer", json!({ "aba": id }));
        assert_eq!(e2["abas"][0]["maxRow"], 0);
        assert_eq!(e2["abas"][0]["podeRefazer"], true);
    }

    #[test]
    fn o_estilo_atravessa_sem_perder_o_falso() {
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        chamar(
            &mut p,
            "estilo_aplicar",
            json!({ "aba": id, "area": { "r0": 0, "c0": 0, "r1": 0, "c1": 0 },
                    "patch": { "bold": false, "borders": { "top": true, "color": "#000" } } }),
        );
        let lista = chamar(&mut p, "estilos", json!({ "aba": id, "indices": [0] }));
        assert_eq!(lista[0], json!({ "bold": false, "borders": { "top": true, "color": "#000" } }));
    }

    /// Passo 1 da migração (23/09/2026): localizar, substituir, estatística,
    /// região, AutoSoma e preencher saíram do JavaScript. A fórmula que lê
    /// outra aba só calcula com a pasta em volta — a busca pelo valor tem de
    /// achar o resultado dela.
    #[test]
    fn localizar_substituir_e_preencher_moram_no_rust() {
        let mut p = Ponte::default();
        let e = chamar(&mut p, "pasta_estado", json!({}));
        let id = e["abas"][0]["id"].as_u64().unwrap();
        let e = chamar(&mut p, "aba_inserir", json!({ "nome": "Dados" }));
        let dados = e["id"].as_u64().unwrap();
        chamar(&mut p, "celula_gravar", json!({ "aba": dados, "linha": 0, "coluna": 0, "raw": "21" }));
        for (l, raw) in [(0, "maçã"), (1, "=Dados!A1*2"), (3, "Maçã verde"), (5, "1"), (6, "2")] {
            chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": l, "coluna": 0, "raw": raw }));
        }
        let opcoes = json!({ "matchCase": false, "wholeCell": false, "inFormulas": false });
        let r = chamar(&mut p, "localizar", json!({ "aba": id, "consulta": "MAÇÃ", "opcoes": opcoes, "limite": 1,
                                                   "apos": { "linha": 0, "coluna": 0 } }));
        assert_eq!(r["total"], 2);
        assert_eq!(r["achadas"], json!([[0, 0]]), "a lista respeita o teto");
        assert_eq!(r["proxima"], json!({ "linha": 3, "coluna": 0, "indice": 1 }));
        let r = chamar(&mut p, "localizar", json!({ "aba": id, "consulta": "maçã", "opcoes": opcoes,
                                                   "apos": { "linha": 0, "coluna": 0, "reverso": true } }));
        assert_eq!(r["proxima"], json!({ "linha": 3, "coluna": 0, "indice": 1 }), "para trás dá a volta");
        let inteira = json!({ "matchCase": false, "wholeCell": true, "inFormulas": false });
        let r = chamar(&mut p, "localizar", json!({ "aba": id, "consulta": "42", "opcoes": inteira }));
        assert_eq!(r["total"], 1, "o valor da fórmula que lê outra aba");

        // Só a célula pedida, e depois tudo — um Ctrl+Z por vez.
        let r = chamar(&mut p, "substituir", json!({ "aba": id, "consulta": "maçã", "troca": "pera", "opcoes": opcoes,
                                                    "celula": { "linha": 3, "coluna": 0 } }));
        assert_eq!(r["total"], 1);
        let s = &p.pasta.aba(0).unwrap().sheet;
        assert_eq!((s.get_raw(0, 0), s.get_raw(3, 0)), ("maçã".to_string(), "pera verde".to_string()));
        chamar(&mut p, "desfazer", json!({ "aba": id }));
        assert_eq!(p.pasta.aba(0).unwrap().sheet.get_raw(3, 0), "Maçã verde");

        let st = chamar(&mut p, "estatisticas_coluna", json!({ "aba": id, "area": { "r0": 0, "c0": 0, "r1": 1048575, "c1": 0 } }));
        assert_eq!((st["count"].as_u64(), st["numbers"].as_u64(), st["sum"].as_f64()), (Some(5), Some(3), Some(45.0)));
        let reg = chamar(&mut p, "regiao_atual", json!({ "aba": id, "area": { "r0": 5, "c0": 0, "r1": 5, "c1": 0 } }));
        assert_eq!(reg, json!({ "r0": 5, "c0": 0, "r1": 6, "c1": 0 }));
        let soma = chamar(&mut p, "autosoma", json!({ "aba": id, "linha": 7, "coluna": 0 }));
        assert_eq!(soma, json!("=SOMA(A6:A7)"));

        let r = chamar(&mut p, "preencher", json!({ "aba": id, "origem": { "r0": 5, "c0": 0, "r1": 6, "c1": 0 },
                                                   "destino": { "r0": 5, "c0": 0, "r1": 9, "c1": 0 } }));
        assert_eq!(r["total"], 3);
        assert_eq!(p.pasta.aba(0).unwrap().sheet.get_raw(9, 0), "5");
        assert!(r["estado"]["abas"][0]["podeDesfazer"].as_bool().unwrap());
    }

    /// O gráfico só relê a planilha quando a edição toca o intervalo dele
    /// (`notarMudanca`, processo crítico). Para isso o estado leva as células
    /// tocadas — e `null` quando não se sabe, que obriga a reler.
    #[test]
    fn o_estado_leva_as_celulas_tocadas() {
        let mut p = Ponte::default();
        let e = chamar(&mut p, "pasta_estado", json!({}));
        let id = e["abas"][0]["id"].as_u64().unwrap();
        assert!(e["abas"][0]["tocadas"].is_null(), "aba nova: não se sabe o que mudou");
        chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 0, "coluna": 0, "raw": "1" }));
        chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 0, "coluna": 1, "raw": "=A1*2" }));
        // Muda A1: a fórmula B1, que lê A1, vai junto.
        let e = chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 0, "coluna": 0, "raw": "5" }));
        assert_eq!(e["abas"][0]["tocadas"], json!([0, 0, 0, 1]));
        // Comando que não muda nada não fala em tocadas.
        let e = chamar(&mut p, "pasta_estado", json!({}));
        assert!(e["abas"][0].get("tocadas").is_none());
        // Ordenar mexe em massa: "não sei onde".
        let r = chamar(&mut p, "ordenar", json!({ "aba": id, "area": { "r0": 0, "c0": 0, "r1": 0, "c1": 1 },
                                                  "chaves": [{ "coluna": 0, "crescente": true }] }));
        let estado = if r.get("estado").is_some() { &r["estado"] } else { &r };
        assert!(estado["abas"][0]["tocadas"].is_null(), "ordenar: {}", estado["abas"][0]["tocadas"]);
        // E o próximo preciso volta a ser preciso.
        let e = chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 9, "coluna": 3, "raw": "x" }));
        assert_eq!(e["abas"][0]["tocadas"], json!([9, 3]));
    }

    /// `IMPORTARDADOS` de ponta a ponta pela ponte: a célula mostra
    /// `#CALCULANDO!`, a pergunta sai uma vez só (duas células, uma tarefa),
    /// a resposta crua derrama, quem depende acompanha, e o "atualizar"
    /// pergunta de novo.
    #[test]
    fn tarefas_assincronas_pela_ponte() {
        use feathersheet_core::Value;
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        let url = "https://exemplo.test/v.csv";
        chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 0, "coluna": 0, "raw": format!("=IMPORTARDADOS(\"{url}\")") }));
        chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 0, "coluna": 5, "raw": format!("=IMPORTARDADOS(\"{url}\")") }));
        chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 9, "coluna": 0, "raw": "=NÚM.CARACT(A1)*2" }));
        let calculando = |v: Value| matches!(v, Value::Error(e) if e.code == feathersheet_core::celulas::values::ErrorCode::Calculating);
        assert!(calculando(p.pasta.valor(0, 0, 0)));
        assert!(calculando(p.pasta.valor(0, 0, 5)));
        let r = chamar(&mut p, "tarefas_pendentes", json!({}));
        let chave = format!("web:dados:{url}\u{0}");
        assert_eq!(r["pendentes"], json!([{ "aba": id, "chave": chave }]));
        assert_eq!(chamar(&mut p, "tarefas_pendentes", json!({}))["pendentes"], json!([]), "em curso não volta");

        let r = chamar(&mut p, "tarefa_resolver", json!({ "aba": id, "chave": chave, "texto": "Produto,Qtd\nCaneta,12\n" }));
        assert!(r["estado"]["abas"][0]["versao"].is_number());
        assert_eq!(p.pasta.valor(0, 0, 0), Value::Text("Produto".into()));
        // A tabela chega inteira, mas o `sheet-core` ainda não derrama
        // matriz nenhuma (nem `=SEQUÊNCIA(3)`): só a âncora aparece. Quando
        // o derramamento entrar, `B2` aqui é 12.
        assert_eq!(p.pasta.valor(0, 0, 5), Value::Text("Produto".into()), "a outra célula divide a resposta");
        assert_eq!(p.pasta.valor(0, 9, 0), Value::Number(14.0), "quem depende acompanha");

        // Atualizar: volta a calcular e a pergunta sai de novo.
        let r = chamar(&mut p, "tarefas_atualizar", json!({}));
        assert_eq!(r["pendentes"], json!([{ "aba": id, "chave": chave }]));
        assert!(calculando(p.pasta.valor(0, 0, 0)));
        chamar(&mut p, "tarefa_resolver", json!({ "aba": id, "chave": chave, "erro": "O servidor respondeu 500" }));
        match p.pasta.valor(0, 0, 0) {
            Value::Error(e) => {
                assert_eq!(e.code, feathersheet_core::celulas::values::ErrorCode::Error);
                assert_eq!(e.detail.as_deref(), Some("O servidor respondeu 500"));
            }
            outro => panic!("esperava #ERROR!, veio {outro:?}"),
        }
    }

    /// A função criada por macro: sem registro é `#NAME?`; registrada, a
    /// fórmula pergunta com os argumentos na chave, e a resposta pronta
    /// (`valor`) entra. Redefinir a função refaz a pergunta.
    #[test]
    fn funcao_de_macro_pela_ponte() {
        use feathersheet_core::Value;
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 0, "coluna": 0, "raw": "21" }));
        chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 0, "coluna": 1, "raw": "=dobro(A1; \"x\")" }));
        assert!(matches!(p.pasta.valor(0, 0, 1), Value::Error(e) if e.code == feathersheet_core::celulas::values::ErrorCode::Name));
        chamar(&mut p, "funcoes_externas_definir", json!({ "nomes": ["dobro"] }));
        assert_eq!(chamar(&mut p, "funcoes_externas", json!({}))["nomes"], json!(["DOBRO"]));
        p.pasta.valor(0, 0, 1);
        let r = chamar(&mut p, "tarefas_pendentes", json!({}));
        let chave = "macro:DOBRO\u{0}[21,\"x\"]";
        assert_eq!(r["pendentes"], json!([{ "aba": id, "chave": chave }]));
        chamar(&mut p, "tarefa_resolver", json!({ "aba": id, "chave": chave, "valor": 42 }));
        assert_eq!(p.pasta.valor(0, 0, 1), Value::Number(42.0));

        // Corpo novo: a resposta velha sai, e a pergunta volta.
        chamar(&mut p, "funcoes_externas_definir", json!({ "nomes": ["DOBRO"], "esquecer": ["dobro"] }));
        assert_eq!(chamar(&mut p, "tarefas_pendentes", json!({}))["pendentes"], json!([{ "aba": id, "chave": chave }]));
        chamar(&mut p, "tarefa_resolver", json!({ "aba": id, "chave": chave, "valor": { "erro": "#VALUE!", "detalhe": "quebrou" } }));
        assert!(matches!(p.pasta.valor(0, 0, 1), Value::Error(e) if e.code == feathersheet_core::celulas::values::ErrorCode::Value));

        // Sem a função, volta a ser `#NAME?`.
        chamar(&mut p, "funcoes_externas_definir", json!({ "nomes": [] }));
        assert!(matches!(p.pasta.valor(0, 0, 1), Value::Error(e) if e.code == feathersheet_core::celulas::values::ErrorCode::Name));
    }

    /// O último valor conhecido vai no `.fshz` e volta sem perguntar de novo.
    #[test]
    fn o_ultimo_valor_vai_no_arquivo() {
        use feathersheet_core::Value;
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 0, "coluna": 0, "raw": "=BUSCARTEXTO(\"https://x.test/a\")" }));
        p.pasta.valor(0, 0, 0);
        chamar(&mut p, "tarefas_pendentes", json!({}));
        chamar(&mut p, "tarefa_resolver", json!({ "aba": id, "chave": "web:texto:https://x.test/a", "texto": "olá" }));
        let bytes = p.gravar_fshz(0).unwrap();
        let mut outra = Ponte::default();
        outra.abrir_fshz(&bytes).unwrap();
        assert_eq!(outra.pasta.valor(0, 0, 0), Value::Text("olá".into()));
        assert_eq!(chamar(&mut outra, "tarefas_pendentes", json!({}))["pendentes"], json!([]), "abrir não sai buscando");
    }

    /// O exemplo do usuário (24/09/2026): `a b 3`, `a c 5` no compacto, e
    /// `a` recolhido vira uma linha com o 8. O mapa diz o que é cada linha.
    #[test]
    fn dinamica_compacta_recolhe_pela_ponte() {
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        chamar(&mut p, "colar", json!({ "aba": id, "linha": 0, "coluna": 0, "desfazer": false,
            "linhas": [["G", "I", "V"], ["a", "b", "3"], ["a", "c", "5"]] }));
        let aplicar = |p: &mut Ponte, recolhidos: Json| {
            chamar(p, "dinamica_aplicar", json!({
                "aba": id, "origemAba": id, "linha": 0, "coluna": 5,
                "limpar": { "r0": 0, "c0": 5, "r1": 20, "c1": 8 },
                "config": { "origem": { "r0": 0, "c0": 0, "r1": 2, "c1": 2 }, "linhas": [0, 1], "colunas": [],
                            "valores": [{ "coluna": 2, "op": "soma" }],
                            "opcoes": { "layout": "compacto", "subtotais": true, "recolhidos": recolhidos } }
            }))
        };
        let r = aplicar(&mut p, json!([]));
        assert_eq!(r["mapa"], json!([["c", 0, []], ["g", 0, [[0, "a", false]]], ["i", 1, []], ["i", 1, []], ["t", 0, []]]));
        assert_eq!(r["rotulos"], 1);
        let s = &p.pasta.aba(0).unwrap().sheet;
        assert_eq!((s.get_raw(1, 5), s.get_raw(1, 6), s.get_raw(2, 5)), ("a".into(), "8".into(), "b".into()));
        let r = aplicar(&mut p, json!(["a"]));
        assert_eq!(r["altura"], 3);
        assert_eq!(r["mapa"][1], json!(["g", 0, [[0, "a", true]]]));
        let s = &p.pasta.aba(0).unwrap().sheet;
        assert_eq!((s.get_raw(1, 6), s.get_raw(2, 5), s.get_raw(3, 5)), ("8".into(), "Total Geral".into(), "".into()));
    }

    /// Duas pastas no disco, como duas janelas do FeatherSheet: a A cita a B
    /// (`'[plan_b.plz]Planilha1'!A1`), e quando a B é regravada a A acompanha.
    #[test]
    fn vinculo_com_outro_arquivo_pelo_disco() {
        let dir = std::env::temp_dir().join(format!("feathersheet-vinculo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let gravar_b = |valor: &str| {
            let mut b = Ponte::default();
            let id = chamar(&mut b, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
            chamar(&mut b, "celula_gravar", json!({ "aba": id, "linha": 0, "coluna": 0, "raw": valor }));
            let bytes = b.gravar_fshz(0).unwrap();
            std::fs::write(dir.join("plan_b.plz"), bytes).unwrap();
        };
        gravar_b("10");
        let mut a = Ponte::default();
        let id = chamar(&mut a, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        chamar(&mut a, "celula_gravar", json!({ "aba": id, "linha": 0, "coluna": 0, "raw": "='[plan_b.plz]Planilha1'!A1*2" }));
        // Antes de carregar, #REF! — e o cálculo anota o pedido.
        assert!(matches!(a.pasta.valor(0, 0, 0), feathersheet_core::Value::Error(_)));
        let base = dir.to_string_lossy().into_owned();
        let r = chamar(&mut a, "vinculos_atualizar", json!({ "base": base }));
        assert_eq!(r["mudou"], true);
        assert_eq!(a.pasta.valor(0, 0, 0), feathersheet_core::Value::Number(20.0));
        // Nada mudou no disco: nada a fazer.
        assert_eq!(chamar(&mut a, "vinculos_atualizar", json!({ "base": base }))["mudou"], false);
        // A B foi regravada (com outra data de modificação).
        gravar_b("7");
        let f = std::fs::File::options().write(true).open(dir.join("plan_b.plz")).unwrap();
        f.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5)).unwrap();
        assert_eq!(chamar(&mut a, "vinculos_atualizar", json!({ "base": base }))["mudou"], true);
        assert_eq!(a.pasta.valor(0, 0, 0), feathersheet_core::Value::Number(14.0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// As duas janelas abertas: a B nem foi gravada, e o valor vai de uma
    /// para a outra pelo canal (exportar na B, instalar na A), na hora da
    /// edição. Com a B ao vivo, um arquivo velho no disco não passa por cima.
    #[test]
    fn vinculo_ao_vivo_sem_passar_pelo_disco() {
        let dir = std::env::temp_dir().join(format!("feathersheet-vivo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let caminho_b = dir.join("plan_b.plz").to_string_lossy().into_owned();
        let _ = std::fs::remove_file(&caminho_b);
        let base = dir.to_string_lossy().into_owned();

        let mut b = Ponte::default();
        let id_b = chamar(&mut b, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        chamar(&mut b, "celula_gravar", json!({ "aba": id_b, "linha": 0, "coluna": 0, "raw": "10" }));

        let mut a = Ponte::default();
        let id_a = chamar(&mut a, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        chamar(&mut a, "celula_gravar", json!({ "aba": id_a, "linha": 0, "coluna": 0, "raw": "='[plan_b.plz]Planilha1'!A1*2" }));
        let _ = a.pasta.valor(0, 0, 0);
        // Sem arquivo no disco: nada carrega, mas o caminho fica conhecido.
        let r = chamar(&mut a, "vinculos_atualizar", json!({ "base": base }));
        assert_eq!(r["mudou"], false);
        assert_eq!(r["caminhos"], json!([caminho_b]));

        // A B responde com tudo.
        let tudo = chamar(&mut b, "vinculo_exportar", json!({}));
        chamar(&mut a, "vinculo_retrato", json!({ "arquivo": caminho_b, "abas": tudo["abas"] }));
        assert_eq!(a.pasta.valor(0, 0, 0), feathersheet_core::Value::Number(20.0));

        // Uma edição na B: só a célula tocada vai.
        chamar(&mut b, "celula_gravar", json!({ "aba": id_b, "linha": 0, "coluna": 0, "raw": "=1/0" }));
        let um = chamar(&mut b, "vinculo_exportar", json!({ "aba": "Planilha1", "celulas": [[0, 0]] }));
        let celulas = um["abas"][0]["celulas"].clone();
        assert_eq!(celulas, json!([[0, 0, { "erro": "#DIV/0!" }]]));
        chamar(&mut b, "celula_gravar", json!({ "aba": id_b, "linha": 0, "coluna": 0, "raw": "33" }));
        let um = chamar(&mut b, "vinculo_exportar", json!({ "aba": "Planilha1", "celulas": [[0, 0]] }));
        let r = chamar(&mut a, "vinculo_valores", json!({ "arquivo": caminho_b, "aba": "Planilha1", "celulas": um["abas"][0]["celulas"] }));
        assert_eq!(r["mudou"], true);
        assert_eq!(a.pasta.valor(0, 0, 0), feathersheet_core::Value::Number(66.0));

        // Um .fshz velho aparece no disco (com 10): ao vivo, não é relido.
        let mut velha = Ponte::default();
        let idv = chamar(&mut velha, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        chamar(&mut velha, "celula_gravar", json!({ "aba": idv, "linha": 0, "coluna": 0, "raw": "10" }));
        std::fs::write(&caminho_b, velha.gravar_fshz(0).unwrap()).unwrap();
        chamar(&mut a, "vinculos_atualizar", json!({ "base": base, "aoVivo": [caminho_b] }));
        assert_eq!(a.pasta.valor(0, 0, 0), feathersheet_core::Value::Number(66.0), "o disco velho não ganha do ao vivo");
        // A B fechou (sem ao vivo): aí o disco vale.
        chamar(&mut a, "vinculos_atualizar", json!({ "base": base }));
        assert_eq!(a.pasta.valor(0, 0, 0), feathersheet_core::Value::Number(20.0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mover_intervalo_pela_ponte() {
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 0, "coluna": 0, "raw": "7" }));
        chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 0, "coluna": 1, "raw": "=A1*2" }));
        let r = chamar(&mut p, "mover_intervalo", json!({ "aba": id, "origem": { "r0": 0, "c0": 0, "r1": 0, "c1": 0 },
                                                       "linhas": 3, "colunas": 2 }));
        assert_eq!(r["movido"], true);
        // Por cima de B1 (a fórmula, agora =C4*2): pergunta antes.
        let r = chamar(&mut p, "mover_intervalo", json!({ "aba": id, "origem": { "r0": 3, "c0": 2, "r1": 3, "c1": 2 },
                                                       "linhas": -3, "colunas": -1 }));
        assert_eq!((r["ocupado"].as_bool(), r["movido"].as_bool()), (Some(true), Some(false)));
        let s = &p.pasta.aba(0).unwrap().sheet;
        assert_eq!((s.get_raw(0, 0), s.get_raw(3, 2), s.get_raw(0, 1)), ("".into(), "7".into(), "=C4*2".into()));
        assert!(chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["podeDesfazer"].as_bool().unwrap());
    }

    /// Passo 2 da migração: a formatação condicional avaliada no Rust, uma
    /// ida por quadro. Inclui a fórmula personalizada (as "Cores alternadas")
    /// e a escala, que se refaz quando um valor muda.
    #[test]
    fn a_formatacao_condicional_pinta_pelo_rust() {
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        for (l, raw) in [(0, "0"), (1, "5"), (2, "10")] {
            chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": l, "coluna": 0, "raw": raw }));
        }
        let regras = json!([
            { "id": "alt", "ranges": [{ "r0": 0, "c0": 1, "r1": 3, "c1": 1 }],
              "condition": { "kind": "cell", "op": "formula", "values": ["=ÉPAR(LIN())"], "style": { "bg": "#f3f3f3" } } },
            { "id": "esc", "ranges": [{ "r0": 0, "c0": 0, "r1": 2, "c1": 0 }],
              "condition": { "kind": "colorScale", "min": { "type": "min", "color": "#000000" }, "max": { "type": "max", "color": "#ffffff" } } },
            { "id": "?", "ranges": [], "condition": { "kind": "cell", "op": "operadorQueNaoExiste" } },
        ]);
        chamar(&mut p, "metadados_definir", json!({ "aba": id, "r": regras }));
        let bloco = |p: &mut Ponte| chamar(p, "condicional_bloco", json!({ "aba": id, "r0": 0, "c0": 0, "r1": 3, "c1": 1 }));
        let r = bloco(&mut p);
        assert_eq!(r[0], json!([0, 0, { "style": { "bg": "#000000" } }]));
        assert!(r.as_array().unwrap().contains(&json!([1, 1, { "style": { "bg": "#f3f3f3" } }])), "LIN() par pinta");
        assert!(r.as_array().unwrap().contains(&json!([1, 0, { "style": { "bg": "#808080" } }])));
        assert!(!r.as_array().unwrap().iter().any(|x| x[0] == 0 && x[1] == 1), "linha ímpar não pinta");
        // Mudou o topo da escala: a estatística se refaz.
        chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 2, "coluna": 0, "raw": "20" }));
        assert!(bloco(&mut p).as_array().unwrap().contains(&json!([1, 0, { "style": { "bg": "#404040" } }])));
    }

    #[test]
    fn a_validacao_verifica_o_valor_pela_ponte() {
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        // Lista de onde a validação por intervalo lê as opções.
        for (l, raw) in [(0, "azul"), (1, "verde")] {
            chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": l, "coluna": 5, "raw": raw }));
        }
        let validacoes = json!([
            { "ranges": [{ "r0": 0, "c0": 0, "r1": 2, "c1": 0 }],
              "regra": { "tipo": "numero", "op": "maior", "a": 10.0 }, "quando": "recusar",
              "mensagem": "precisa passar de 10" },
            { "ranges": [{ "r0": 0, "c0": 1, "r1": 0, "c1": 1 }],
              "regra": { "tipo": "intervalo", "origem": { "r0": 0, "c0": 5, "r1": 1, "c1": 5 } },
              "quando": "avisar" },
        ]);
        chamar(&mut p, "metadados_definir", json!({ "aba": id, "va": validacoes }));
        let verificar = |p: &mut Ponte, r: u32, c: u32, valor: &str| {
            chamar(p, "validacao_verificar", json!({ "aba": id, "r": r, "c": c, "valor": valor }))
        };

        // Recusa (número): abaixo do mínimo, quando é "recusar".
        let r = verificar(&mut p, 1, 0, "5");
        assert_eq!(r["ok"], false);
        assert_eq!(r["quando"], "recusar");
        assert_eq!(r["mensagem"], "precisa passar de 10");
        // Passa a regra.
        assert_eq!(verificar(&mut p, 1, 0, "50")["ok"], true);
        // Vazio é sempre válido, mesmo dentro do intervalo da regra.
        assert_eq!(verificar(&mut p, 1, 0, "")["ok"], true);
        // Fora de qualquer intervalo de validação: sempre ok.
        assert_eq!(verificar(&mut p, 9, 9, "qualquer coisa")["ok"], true);

        // Lista vinda de células (intervalo): "avisar", não "recusar".
        let r = verificar(&mut p, 0, 1, "amarelo");
        assert_eq!(r["ok"], false);
        assert_eq!(r["quando"], "avisar");
        assert_eq!(verificar(&mut p, 0, 1, "verde")["ok"], true);
    }

    #[test]
    fn celula_gravar_recusa_digitar_valor_invalido_numa_celula_com_regra_recusar() {
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        chamar(&mut p, "metadados_definir", json!({ "aba": id, "va": [
            { "ranges": [{ "r0": 0, "c0": 0, "r1": 0, "c1": 0 }],
              "regra": { "tipo": "numero", "op": "maior", "a": 10.0 }, "quando": "recusar",
              "mensagem": "precisa passar de 10" },
        ] }));
        let versao_antes = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["versao"].clone();
        let erro = p.chamar("celula_gravar", &serde_json::to_vec(&json!({
            "aba": id, "linha": 0, "coluna": 0, "raw": "5",
        })).unwrap());
        assert!(erro.is_err());
        assert_eq!(erro.unwrap_err(), "precisa passar de 10");
        let estado = chamar(&mut p, "pasta_estado", json!({}));
        assert_eq!(estado["abas"][0]["versao"], versao_antes, "nada foi escrito");
        chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": 0, "coluna": 0, "raw": "50" }));
    }

    #[test]
    fn colar_recusa_digitar_valor_invalido_numa_celula_com_regra_recusar() {
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        chamar(&mut p, "metadados_definir", json!({ "aba": id, "va": [
            { "ranges": [{ "r0": 0, "c0": 0, "r1": 0, "c1": 0 }],
              "regra": { "tipo": "numero", "op": "maior", "a": 10.0 }, "quando": "recusar",
              "mensagem": "precisa passar de 10" },
        ] }));
        let versao_antes = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["versao"].clone();
        // Digitar (colar 1x1) um valor que não passa: recusado, nada muda.
        let erro = p.chamar("colar", &serde_json::to_vec(&json!({
            "aba": id, "linha": 0, "coluna": 0, "linhas": [["5"]], "desfazer": true,
        })).unwrap());
        assert!(erro.is_err());
        assert_eq!(erro.unwrap_err(), "precisa passar de 10");
        let estado = chamar(&mut p, "pasta_estado", json!({}));
        assert_eq!(estado["abas"][0]["versao"], versao_antes, "nada foi escrito");
        // Um valor que passa entra normalmente.
        chamar(&mut p, "colar", json!({ "aba": id, "linha": 0, "coluna": 0, "linhas": [["50"]], "desfazer": true }));
        // Colar em lote (mais de uma célula) não passa pelo freio de "colar".
        chamar(&mut p, "colar", json!({ "aba": id, "linha": 0, "coluna": 0, "linhas": [["5"], ["6"]], "desfazer": false }));
    }

    #[test]
    fn comando_desconhecido_e_aba_errada_viram_erro_e_nao_panico() {
        let mut p = Ponte::default();
        assert!(p.chamar("nao_existe", b"{}").is_err());
        assert!(p.chamar("celula_gravar", br#"{"aba":999,"linha":0,"coluna":0,"raw":"x"}"#).is_err());
        assert!(p.chamar("celula_gravar", b"isto nao e json").is_err());
    }
    /// Abrir e gravar `.fshz` acontecem dentro da ponte: as células não
    /// atravessam, e o que é metadado vai e volta sem perder nada — inclusive
    /// o apagar (um gráfico removido não pode voltar no arquivo).
    #[test]
    fn fshz_ida_e_volta_pela_ponte() {
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        for r in 0..300u32 {
            chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": r, "coluna": 0, "raw": (r + 1).to_string() }));
            chamar(&mut p, "celula_gravar", json!({ "aba": id, "linha": r, "coluna": 1, "raw": format!("=A{}*2", r + 1) }));
        }
        chamar(
            &mut p,
            "metadados_definir",
            json!({ "aba": id, "tc": "#f00", "w": [[1, 150]], "g": [{ "id": "g1" }], "mc": [{ "r0": 0, "c0": 0, "r1": 0, "c1": 5 }] }),
        );
        let bytes = p.chamar("pasta_gravar_fshz", br#"{"ativa":0}"#).unwrap();

        let mut q = Ponte::default();
        let r: Json = serde_json::from_slice(&q.chamar("pasta_abrir_fshz", &bytes).unwrap()).unwrap();
        assert_eq!(r["estado"]["abas"][0]["maxRow"], 299);
        assert_eq!(r["metadados"][0]["tc"], "#f00");
        assert_eq!(r["metadados"][0]["w"], json!([[1, 150]]));
        assert_eq!(r["metadados"][0]["g"], json!([{ "id": "g1" }]));
        assert_eq!(r["metadados"][0]["mc"], json!([{ "r0": 0, "c0": 0, "r1": 0, "c1": 5 }]), "as mesclas voltam");
        let s = &q.pasta.aba(0).unwrap().sheet;
        assert_eq!(s.get_raw(99, 1), "=A100*2");

        // Sem gráfico agora: o arquivo também fica sem.
        let id2 = r["estado"]["abas"][0]["id"].as_u64().unwrap();
        chamar(&mut q, "metadados_definir", json!({ "aba": id2, "tc": null }));
        let bytes2 = q.chamar("pasta_gravar_fshz", b"{}").unwrap();
        let mut t = Ponte::default();
        let r2: Json = serde_json::from_slice(&t.chamar("pasta_abrir_fshz", &bytes2).unwrap()).unwrap();
        assert_eq!(r2["metadados"][0]["g"], Json::Null);
        assert_eq!(r2["metadados"][0]["tc"], Json::Null);
    }
    #[test]
    fn csv_vira_aba_nova_sem_desfazer() {
        let mut p = Ponte::default();
        let r = chamar(&mut p, "aba_de_csv", json!({ "nome": "dados", "texto": "a;b\n1,5;=A2*2\n\"x;y\";7\n" }));
        assert_eq!(r["linhas"], 3);
        let aba = r["estado"]["abas"].as_array().unwrap().iter().find(|a| a["id"] == r["id"]).unwrap().clone();
        assert_eq!(aba["nome"], "dados");
        assert_eq!(aba["maxRow"], 2);
        assert_eq!(aba["podeDesfazer"], false);
        let s = &p.pasta.aba(1).unwrap().sheet;
        assert_eq!(s.get_value(1, 0), feathersheet_core::celulas::values::Value::Number(1.5));
        assert_eq!(s.get_value(1, 1), feathersheet_core::celulas::values::Value::Number(3.0));
        assert_eq!(s.get_raw(2, 0), "x;y");
    }
    #[test]
    fn pasta_do_formato_do_load_json() {
        let livro = json!({
            "active": 1,
            "names": [{ "name": "Preco", "rect": { "r0": 0, "c0": 0, "r1": 0, "c1": 0 } }],
            "sheets": [
                { "name": "A", "cells": { "0": { "raw": "10", "style": { "bold": true } }, "1": { "raw": "=Preco*2" } },
                  "colWidths": [[0, 120]], "rowHeights": [], "charts": [], "rules": [] },
                { "name": "B", "cells": {} }
            ]
        });
        let mut p = Ponte::default();
        let r: Json = serde_json::from_slice(&p.instalar(PastaAberta::de_json(&livro)).unwrap()).unwrap();
        assert_eq!(r["ativa"], 1);
        assert_eq!(r["estado"]["abas"].as_array().unwrap().len(), 2);
        assert_eq!(r["metadados"][0]["w"], json!([[0, 120]]));
        assert_eq!(r["metadados"][0]["g"], Json::Null);
        assert_eq!(r["nomes"][0]["name"], "Preco");
        assert_eq!(p.pasta.valor(0, 0, 1), feathersheet_core::celulas::values::Value::Number(20.0));
        assert!(p.pasta.aba(0).unwrap().sheet.get_style(0, 0).is_some());
    }
    #[test]
    fn o_rodape_conta_na_ponte_sem_linha_escondida() {
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        chamar(&mut p, "colar", json!({ "aba": id, "linha": 0, "coluna": 0, "linhas": [["1"], ["2"], ["x"], ["4"], ["=A1+A2"]], "desfazer": false }));
        let area = json!({ "r0": 0, "c0": 0, "r1": 1048575, "c1": 0 });
        let r = chamar(&mut p, "selecao_contar", json!({ "aba": id, "area": area }));
        assert_eq!(r, json!({ "count": 5, "numbers": 4, "sum": 10.0, "min": 1.0, "max": 4.0 }));
        // O filtro esconde as linhas 1 e 3 (o "2" e o "4").
        chamar(&mut p, "ocultas_definir", json!({ "aba": id, "faixas": [[3, 3], [1, 1]] }));
        let r = chamar(&mut p, "selecao_contar", json!({ "aba": id, "area": area }));
        assert_eq!(r, json!({ "count": 3, "numbers": 2, "sum": 4.0, "min": 1.0, "max": 3.0 }));
        // Pasta nova: as faixas da velha não valem mais.
        chamar(&mut p, "pasta_nova", json!({}));
        let vazia = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        let r = chamar(&mut p, "selecao_contar", json!({ "aba": vazia, "area": area }));
        assert_eq!(r["count"], 0);
        assert_eq!(r["min"], Json::Null);
    }
    #[test]
    fn a_dinamica_calcula_e_escreve_dentro_do_rust() {
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        let linhas = json!([["Tipo", "Valor"], ["a", "1"], ["b", "2"], ["a", "3"]]);
        chamar(&mut p, "colar", json!({ "aba": id, "linha": 0, "coluna": 0, "linhas": linhas, "desfazer": false }));
        let r = chamar(
            &mut p,
            "dinamica_aplicar",
            json!({
                "aba": id, "origemAba": id, "linha": 0, "coluna": 4,
                "limpar": { "r0": 0, "c0": 4, "r1": 49, "c1": 53 },
                "config": { "origem": { "r0": 0, "c0": 0, "r1": 3, "c1": 1 }, "linhas": [0], "colunas": [], "valores": [{ "coluna": 1, "op": "soma" }] }
            }),
        );
        assert!(r["altura"].as_u64().unwrap() >= 3, "{r}");
        let s = &p.pasta.aba(0).unwrap().sheet;
        let mut achou = std::collections::HashMap::new();
        for linha in 0..r["altura"].as_u64().unwrap() as u32 {
            achou.insert(s.get_raw(linha, 4), s.get_value(linha, 5));
        }
        assert_eq!(achou.get("a"), Some(&feathersheet_core::Value::Number(4.0)), "{achou:?}");
        assert_eq!(achou.get("b"), Some(&feathersheet_core::Value::Number(2.0)));
    }
    #[test]
    fn campo_calculado_na_dinamica() {
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        let linhas = json!([["Tipo", "ValorA", "ValorB"], ["a", "1", "10"], ["b", "2", "20"], ["a", "3", "30"]]);
        chamar(&mut p, "colar", json!({ "aba": id, "linha": 0, "coluna": 0, "linhas": linhas, "desfazer": false }));
        // A origem vai até a coluna 3: a 3 é a virtual "A+B".
        let r = chamar(
            &mut p,
            "dinamica_aplicar",
            json!({
                "aba": id, "origemAba": id, "linha": 0, "coluna": 6,
                "config": {
                    "origem": { "r0": 0, "c0": 0, "r1": 3, "c1": 3 }, "linhas": [0], "colunas": [],
                    "valores": [{ "coluna": 3, "op": "soma" }],
                    "calculados": [{ "nome": "A+B", "formula": "ValorA + ValorB" }]
                }
            }),
        );
        let s = &p.pasta.aba(0).unwrap().sheet;
        let mut achou = std::collections::HashMap::new();
        for linha in 0..r["altura"].as_u64().unwrap() as u32 {
            achou.insert(s.get_raw(linha, 6), s.get_value(linha, 7));
        }
        assert_eq!(achou.get("a"), Some(&feathersheet_core::Value::Number(44.0)), "{achou:?}");
        assert_eq!(achou.get("b"), Some(&feathersheet_core::Value::Number(22.0)));
        // Fórmula com campo que não existe: erro explicado, nada escrito.
        let erro = p
            .chamar(
                "dinamica_aplicar",
                &serde_json::to_vec(&json!({
                    "aba": id, "origemAba": id, "linha": 0, "coluna": 6,
                    "config": { "origem": { "r0": 0, "c0": 0, "r1": 3, "c1": 3 }, "linhas": [0], "colunas": [],
                        "valores": [{ "coluna": 3, "op": "soma" }], "calculados": [{ "nome": "X", "formula": "Nada * 2" }] }
                }))
                .unwrap(),
            )
            .unwrap_err();
        assert!(erro.contains("Nada"), "{erro}");
    }
    #[test]
    fn o_grafico_agrupa_dentro_do_rust() {
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        let linhas = json!([["Status", "Valor"], ["ALTO", "1"], ["BAIXO", "2"], ["ALTO", "3"]]);
        chamar(&mut p, "colar", json!({ "aba": id, "linha": 0, "coluna": 0, "linhas": linhas, "desfazer": false }));
        let d = chamar(
            &mut p,
            "grafico_dados",
            json!({ "aba": id, "config": {
                "id": "g", "type": "column", "title": "T", "range": { "r0": 0, "c0": 0, "r1": 3, "c1": 1 },
                "seriesInColumns": true, "hasHeaders": true, "hasCategoryLabels": true, "aggregate": true,
                "position": { "x": 0, "y": 0, "width": 10, "height": 10 }
            } }),
        );
        let cats: Vec<String> = serde_json::from_value(d["categories"].clone()).unwrap();
        let vals: Vec<f64> = serde_json::from_value(d["series"][0]["values"].clone()).unwrap();
        let mapa: std::collections::HashMap<_, _> = cats.into_iter().zip(vals).collect();
        assert_eq!(mapa.get("ALTO"), Some(&4.0), "{d}");
        assert_eq!(mapa.get("BAIXO"), Some(&2.0));
    }
    #[test]
    fn filtro_e_valores_distintos_na_ponte() {
        let mut p = Ponte::default();
        let id = chamar(&mut p, "pasta_estado", json!({}))["abas"][0]["id"].as_u64().unwrap();
        let linhas = json!([["Status"], ["ALTO"], ["BAIXO"], ["ALTO"], [""], ["NORMAL"]]);
        chamar(&mut p, "colar", json!({ "aba": id, "linha": 0, "coluna": 0, "linhas": linhas, "desfazer": false }));
        let d = chamar(&mut p, "valores_distintos", json!({ "aba": id, "coluna": 0, "r0": 1, "r1": 9_999_999 }));
        assert_eq!(d, json!({ "valores": ["ALTO", "BAIXO", null, "NORMAL"], "cortado": false }));
        let d = chamar(&mut p, "valores_distintos", json!({ "aba": id, "coluna": 0, "r0": 1, "r1": 9_999_999, "limite": 2 }));
        assert_eq!(d["cortado"], true);
        let f = chamar(&mut p, "filtro_avaliar", json!({ "aba": id, "r0": 1, "r1": 9_999_999, "filtros": [{ "col": 0, "lista": ["ALTO"] }] }));
        assert_eq!(f, json!([[2, 2], [4, 5]]));
        let f = chamar(&mut p, "filtro_avaliar", json!({ "aba": id, "r0": 1, "r1": 9_999_999, "filtros": [{ "col": 0, "op": "starts_with", "val": "n" }] }));
        assert_eq!(f, json!([[1, 4]]));
    }

    fn montar_xlsx(partes: &[(&str, &str)]) -> Vec<u8> {
        feathersheet_io::zip_write::zipar(partes.iter().map(|(n, c)| (n.to_string(), c.as_bytes().to_vec())).collect()).unwrap()
    }

    const WORKBOOK_MIN: &str = r#"<?xml version="1.0"?><workbook><sheets><sheet name="Plan1" r:id="rId1"/></sheets></workbook>"#;
    const RELS_MIN: &str = r#"<?xml version="1.0"?><Relationships><Relationship Id="rId1" Target="worksheets/sheet1.xml"/></Relationships>"#;

    /// O `.xlsx` vira pasta viva sem passar pelo JavaScript: os bytes entram
    /// no `pasta_abrir_xlsx` (veio do `src-tauri` com o leitor).
    #[test]
    fn xlsx_vira_pasta_do_rust() {
        let sheet1 = r#"<worksheet><sheetData><row r="1"><c r="A1"><v>21</v></c><c r="B1"><f>A1*2</f><v>42</v></c></row></sheetData></worksheet>"#;
        let bytes = montar_xlsx(&[
            ("xl/workbook.xml", WORKBOOK_MIN),
            ("xl/_rels/workbook.xml.rels", RELS_MIN),
            ("xl/worksheets/sheet1.xml", sheet1),
        ]);
        let mut ponte = Ponte::default();
        ponte.chamar("pasta_abrir_xlsx", &bytes).unwrap();
        assert_eq!(ponte.pasta.aba(0).unwrap().sheet.name, "Plan1");
        assert_eq!(ponte.pasta.aba(0).unwrap().sheet.get_raw(0, 1), "=A1*2");
        assert_eq!(ponte.pasta.valor(0, 0, 1), feathersheet_core::celulas::values::Value::Number(42.0));
    }

    /// Item 19 (23/09/2026): o endereço do hiperlink mora nas relações da
    /// aba, à parte do texto. Tem de chegar à pasta do Rust e sobreviver à
    /// gravação do `.fshz` — o gravador passa pelo `Style` tipado.
    #[test]
    fn o_hiperlink_do_xlsx_chega_e_sobrevive_ao_fshz() {
        let sheet1 = r#"<worksheet xmlns:r="x"><sheetData><row r="1"><c r="A1" t="str"><v>Site</v></c></row><row r="2"><c r="A2" t="str"><v>Doc</v></c></row></sheetData><hyperlinks><hyperlink ref="A1:A3" r:id="rId1"/></hyperlinks></worksheet>"#;
        let rels = r#"<Relationships><Relationship Id="rId1" Target="https://exemplo.com/?a=1&amp;b=2" TargetMode="External"/></Relationships>"#;
        let bytes = montar_xlsx(&[
            ("xl/workbook.xml", WORKBOOK_MIN),
            ("xl/_rels/workbook.xml.rels", RELS_MIN),
            ("xl/worksheets/sheet1.xml", sheet1),
            ("xl/worksheets/_rels/sheet1.xml.rels", rels),
        ]);
        let livro = feathersheet_io::xlsx_read::ler_xlsx_bytes(bytes).unwrap();
        assert_eq!(livro["sheets"][0]["cells"]["0"]["style"]["link"], "https://exemplo.com/?a=1&b=2");
        assert_eq!(livro["sheets"][0]["cells"]["16384"]["style"]["link"], "https://exemplo.com/?a=1&b=2");

        let mut ponte = Ponte::default();
        ponte.instalar(PastaAberta::de_json(&livro)).unwrap();
        let fshz = feathersheet_io::fshz_write::gerar_fshz2(&ponte.retrato(0)).unwrap();
        let reaberta = feathersheet_io::para_workbook::para_workbook(&feathersheet_io::fshz_read::ler_fshz(&fshz).unwrap());
        let s = &reaberta.aba(0).unwrap().sheet;
        assert_eq!(s.get_raw(0, 0), "Site");
        assert_eq!(s.get_style(0, 0).and_then(|e| e.link.clone()).as_deref(), Some("https://exemplo.com/?a=1&b=2"));
        assert_eq!(s.get_style(1, 0).and_then(|e| e.link.clone()).as_deref(), Some("https://exemplo.com/?a=1&b=2"));
    }

}
